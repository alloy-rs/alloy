use crate::{
    provider::watch_from_common::FixedBuf, transport::TransportErrorKind, WatchBlocksFrom,
};
use alloy_consensus::BlockHeader;
use alloy_network::{BlockResponse as _, Network};
use alloy_network_primitives::HeaderResponse;
use alloy_transport::TransportResult;
use async_stream::try_stream;
use futures::{Stream, StreamExt as _};
use std::collections::VecDeque;

const RPC_CONCURRENCY_DEFAULT: usize = 4;
const MAX_REORG_DEPTH_DEFAULT: usize = 64;

/// A builder for streaming blocks from a historical block and continuing indefinitely.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchCanonicalBlocksFrom<N: Network> {
    watch_blocks_from: WatchBlocksFrom<N>,
    rpc_concurrency: usize,
    max_reorg_depth: usize,
}

/// An item emitted by the canonical block stream.
#[derive(Debug, Clone)]
pub enum CanonicalEvent<T> {
    /// A new canonical block to add.
    Added(T),
    /// A canonical block to remove due to a reorg.
    Removed(T),
}

impl<N: Network> WatchCanonicalBlocksFrom<N> {
    pub(crate) const fn new(watch_blocks_from: WatchBlocksFrom<N>) -> Self {
        Self {
            watch_blocks_from,
            rpc_concurrency: RPC_CONCURRENCY_DEFAULT,
            max_reorg_depth: MAX_REORG_DEPTH_DEFAULT,
        }
    }

    /// Sets the number of in-flight `eth_getBlockByNumber` requests.
    pub const fn rpc_concurrency(mut self, rpc_concurrency: usize) -> Self {
        self.rpc_concurrency = if rpc_concurrency == 0 { 1 } else { rpc_concurrency };
        self
    }

    /// Sets the maximum number of canonical blocks retained for reorg detection.
    pub const fn max_reorg_depth(mut self, max_reorg_depth: usize) -> Self {
        self.max_reorg_depth = if max_reorg_depth == 0 { 1 } else { max_reorg_depth };
        self
    }

    /// Converts the builder into a stream of canonical block events.
    pub fn into_stream(
        self,
    ) -> impl Stream<Item = TransportResult<CanonicalEvent<N::BlockResponse>>> + Unpin + 'static
    {
        let Self { watch_blocks_from, rpc_concurrency, max_reorg_depth } = self;
        let rpc_concurrency = rpc_concurrency.max(1);

        try_stream! {
            let mut buffer: FixedBuf<N::BlockResponse> = FixedBuf::new(max_reorg_depth);
            let mut stream = watch_blocks_from.clone().into_stream().buffered(rpc_concurrency);

            while let Some(next) = stream.next().await {
                let next = next?;

                // Contains the replacement chain segment to add.
                // In non-reorg cases this is just `next`.
                let mut front = &next;
                let mut pending_additions = VecDeque::<N::BlockResponse>::new();

                loop {
                    let Some(canonical_tip) = buffer.last() else {
                        break;
                    };

                    let parent_hash = front
                        .header()
                        .parent_hash();

                    // Normal extension of the canonical tip.
                    if parent_hash == canonical_tip.header().hash() {
                        break;
                    }

                    // Reorg that connects to a retained ancestor.
                    if let Some(pos) =
                        buffer.iter().rev().position(|block| block.header().hash() == parent_hash)
                    {
                        for _ in 0..pos {
                            let old =
                                buffer.pop().expect("position is always < canonical buffer length");
                            yield CanonicalEvent::Removed(old);
                        }
                        break;
                    }

                    // Reorg parent was not found in retained history:
                    // remove one canonical tip block and walk one parent block backward.
                    let old = buffer.pop().expect("canonical tip exists");
                    yield CanonicalEvent::Removed(old);

                    let parent_number = front
                        .header()
                        .number()
                        .checked_sub(1)
                        .ok_or_else(|| {
                            TransportErrorKind::custom_str("reorg detected at genesis block")
                        })?;

                    let parent = watch_blocks_from.get_block(parent_number).await?;
                    pending_additions.push_front(parent);
                    front = pending_additions.front().expect("just pushed");

                    // We exhausted retained canonical history before finding a common ancestor.
                    if buffer.last().is_none() {
                        Err(TransportErrorKind::custom_str(
                            "reorg exceeds max_reorg_depth; canonical ancestor not found",
                        ))?;
                    }
                }

                for block in pending_additions {
                    buffer.push(block.clone());
                    yield CanonicalEvent::Added(block);
                }
                buffer.push(next.clone());
                yield CanonicalEvent::Added(next);

            }
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};
    use alloy_eips::BlockNumberOrTag;
    use alloy_primitives::B256;
    use alloy_rpc_types_eth::Block;
    use futures::StreamExt;
    use tokio::time::timeout;

    fn block(number: u64, hash_last_byte: u8, parent_hash_last_byte: u8) -> Block {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block.header.hash = B256::with_last_byte(hash_last_byte);
        block.header.inner.parent_hash = B256::with_last_byte(parent_hash_last_byte);
        block
    }

    #[tokio::test]
    async fn emits_removed_then_added_on_reorg_within_buffer() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        // head + block 1,2,3,4 (where block 4 reorgs from block 2).
        asserter.push_success(&4_u64);
        asserter.push_success(&Some(block(1, 1, 0)));
        asserter.push_success(&Some(block(2, 2, 1)));
        asserter.push_success(&Some(block(3, 3, 2)));
        asserter.push_success(&Some(block(4, 44, 2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(std::time::Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(16)
            .into_stream();

        let first = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let second = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let third = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let fourth = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let fifth = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        match first {
            CanonicalEvent::Added(block) => assert_eq!(block.header.number, 1),
            other => panic!("expected Added(1), got {other:?}"),
        }
        match second {
            CanonicalEvent::Added(block) => assert_eq!(block.header.number, 2),
            other => panic!("expected Added(2), got {other:?}"),
        }
        match third {
            CanonicalEvent::Added(block) => assert_eq!(block.header.number, 3),
            other => panic!("expected Added(3), got {other:?}"),
        }
        match fourth {
            CanonicalEvent::Removed(block) => assert_eq!(block.header.number, 3),
            other => panic!("expected Removed(3), got {other:?}"),
        }
        match fifth {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 4);
                assert_eq!(block.header.hash, B256::with_last_byte(44));
            }
            other => panic!("expected Added(4'), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn emits_error_when_reorg_exceeds_retained_history() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        // Initial old chain blocks 1 -> 2 -> 3.
        asserter.push_success(&4_u64);
        asserter.push_success(&Some(block(1, 1, 0)));
        asserter.push_success(&Some(block(2, 2, 1)));
        asserter.push_success(&Some(block(3, 3, 2)));
        // New block 4 on a different chain (parent 3').
        asserter.push_success(&Some(block(4, 44, 33)));
        // Parent walk fetched manually after reorg detection.
        asserter.push_success(&Some(block(3, 33, 22)));
        asserter.push_success(&Some(block(2, 22, 11)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(std::time::Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(2)
            .into_stream();

        // Added 1, 2, 3.
        for expected in [1_u64, 2, 3] {
            let item = timeout(std::time::Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Removed 3, Removed 2.
        let removed_3 = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let removed_2 = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        match removed_3 {
            CanonicalEvent::Removed(block) => assert_eq!(block.header.number, 3),
            other => panic!("expected Removed(3), got {other:?}"),
        }
        match removed_2 {
            CanonicalEvent::Removed(block) => assert_eq!(block.header.number, 2),
            other => panic!("expected Removed(2), got {other:?}"),
        }

        let err = timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(format!("{err}").contains("max_reorg_depth"));

        // Stream ends after the first error.
        let next = timeout(std::time::Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }
}
