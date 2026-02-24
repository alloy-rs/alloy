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

            'stream: while let Some(next) = stream.next().await {
                let next = next?;

                // Contains the replacement chain segment to add.
                // In non-reorg cases this is just `next`.
                let mut front = &next;
                let mut pending = VecDeque::<N::BlockResponse>::new();

                loop {
                    // First item, carry on as usual.
                    let Some(canonical_tip) = buffer.last() else {
                        break;
                    };

                    let parent_hash = front.header().parent_hash();


                    // Normal extension of the canonical tip.
                    if parent_hash == canonical_tip.header().hash() {
                        break;
                    }

                    // Reorg detected: the new block does not build on the current canonical tip.
                    let height = front.header().number();
                    let canonical_height = canonical_tip.header().number();
                    if canonical_height + 1 == height {
                        // The hashes don't match even though the block numbers are sequential.
                        yield CanonicalEvent::Removed(buffer.pop().expect("position is always < canonical buffer length"));
                        if buffer.len() == 0 {
                            Err(TransportErrorKind::custom_str(
                                "Deep reorg detected; no canonical history retained.",
                            ))?;
                        }
                    }

                    let parent = watch_blocks_from.get_block(height - 1).await?;
                    if parent.header().hash() != parent_hash {
                        // We have hit a second reorg.
                        // This means that `next` is no longer canonical.
                        // Abandon progress and try to work backwards again.
                        continue 'stream;
                    }
                    pending.push_front(parent);
                    front = pending.front().expect("just pushed");
                }

                for block in pending {
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
    use std::time::Duration;
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

        // Old chain: 1 -> 2 -> 3. New chain: 1 -> 2 -> 3' -> 4.
        asserter.push_success(&4_u64);
        asserter.push_success(&Some(block(1, 1, 0)));
        asserter.push_success(&Some(block(2, 2, 1)));
        asserter.push_success(&Some(block(3, 3, 2)));
        asserter.push_success(&Some(block(4, 44, 33)));
        // Backfill: block 3' (hash=33, parent=2) replaces old block 3.
        asserter.push_success(&Some(block(3, 33, 2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(16)
            .into_stream();

        // Added 1, 2, 3.
        for expected in [1_u64, 2, 3] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Removed 3, Added 3', Added 4.
        let removed_3 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_3_prime =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_4 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();

        match removed_3 {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(3));
            }
            other => panic!("expected Removed(3), got {other:?}"),
        }
        match added_3_prime {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(33));
            }
            other => panic!("expected Added(3'), got {other:?}"),
        }
        match added_4 {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 4);
                assert_eq!(block.header.hash, B256::with_last_byte(44));
            }
            other => panic!("expected Added(4), got {other:?}"),
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
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(2)
            .into_stream();

        // Added 1, 2, 3.
        for expected in [1_u64, 2, 3] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Removed 3, Removed 2.
        let removed_3 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let removed_2 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        match removed_3 {
            CanonicalEvent::Removed(block) => assert_eq!(block.header.number, 3),
            other => panic!("expected Removed(3), got {other:?}"),
        }
        match removed_2 {
            CanonicalEvent::Removed(block) => assert_eq!(block.header.number, 2),
            other => panic!("expected Removed(2), got {other:?}"),
        }

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(format!("{err}").contains("Deep reorg detected"));

        // Stream ends after the first error.
        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn backfills_parent_chain_when_reorg_ancestor_is_retained() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        // Old chain: 1 -> 2 -> 3 -> 4.
        asserter.push_success(&5_u64);
        asserter.push_success(&Some(block(1, 1, 0)));
        asserter.push_success(&Some(block(2, 2, 1)));
        asserter.push_success(&Some(block(3, 3, 2)));
        asserter.push_success(&Some(block(4, 4, 3)));
        // New tip block 5 extends 4', so we need to backfill 4' and 3' by number.
        asserter.push_success(&Some(block(5, 5, 44)));
        asserter.push_success(&Some(block(4, 44, 33)));
        asserter.push_success(&Some(block(3, 33, 2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        // Added 1,2,3,4.
        for expected in [1_u64, 2, 3, 4] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Removed 4, Removed 3, Added 3', Added 4', Added 5.
        let removed_4 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let removed_3 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_3_prime =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_4_prime =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_5 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();

        match removed_4 {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, 4);
                assert_eq!(block.header.hash, B256::with_last_byte(4));
            }
            other => panic!("expected Removed(4), got {other:?}"),
        }
        match removed_3 {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(3));
            }
            other => panic!("expected Removed(3), got {other:?}"),
        }
        match added_3_prime {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(33));
            }
            other => panic!("expected Added(3'), got {other:?}"),
        }
        match added_4_prime {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 4);
                assert_eq!(block.header.hash, B256::with_last_byte(44));
            }
            other => panic!("expected Added(4'), got {other:?}"),
        }
        match added_5 {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 5);
                assert_eq!(block.header.hash, B256::with_last_byte(5));
            }
            other => panic!("expected Added(5), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn recovers_when_chain_changes_during_backfill() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        // Old chain: 1 -> 2 -> 3.
        asserter.push_success(&4_u64);
        asserter.push_success(&Some(block(1, 1, 0)));
        asserter.push_success(&Some(block(2, 2, 1)));
        asserter.push_success(&Some(block(3, 3, 2)));
        // Block 4 references parent hash 33 (a new block 3').
        asserter.push_success(&Some(block(4, 4, 33)));
        // Backfill block 3: chain changed again, returns hash 34 instead of 33.
        asserter.push_success(&Some(block(3, 34, 2)));
        // continue 'stream: buffer is now [1, 2], stream advances to next head poll.
        asserter.push_success(&5_u64);
        asserter.push_success(&Some(block(5, 5, 44)));
        // Backfill from block 5 down to buffer tip (block 2).
        asserter.push_success(&Some(block(4, 44, 33)));
        asserter.push_success(&Some(block(3, 33, 2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        // Added 1, 2, 3.
        for expected in [1_u64, 2, 3] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Removed 3 (from the failed first attempt).
        let removed_3 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        match removed_3 {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(3));
            }
            other => panic!("expected Removed(3), got {other:?}"),
        }

        // After recovery: Added 3', Added 4', Added 5.
        let added_3_prime =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_4_prime =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        let added_5 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();

        match added_3_prime {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(33));
            }
            other => panic!("expected Added(3'), got {other:?}"),
        }
        match added_4_prime {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 4);
                assert_eq!(block.header.hash, B256::with_last_byte(44));
            }
            other => panic!("expected Added(4'), got {other:?}"),
        }
        match added_5 {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, 5);
                assert_eq!(block.header.hash, B256::with_last_byte(5));
            }
            other => panic!("expected Added(5), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn clamps_zero_values_for_rpc_concurrency_and_reorg_depth() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&1_u64);
        asserter.push_success(&Some(block(1, 1, 0)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(0)
            .max_reorg_depth(0)
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        match first {
            CanonicalEvent::Added(block) => assert_eq!(block.header.number, 1),
            other => panic!("expected Added(1), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let provider =
            ProviderBuilder::new().connect_mocked_client(alloy_transport::mock::Asserter::new());
        let mut stream = provider.watch_canonical_blocks_from(0).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }
}
