use crate::{transport::TransportErrorKind, WatchBlocksFrom};
use alloy_consensus::BlockHeader;
use alloy_network::{BlockResponse as _, Network};
use alloy_network_primitives::HeaderResponse;
use alloy_transport::TransportResult;
use async_stream::try_stream;
use futures::{Stream, StreamExt as _};
use std::collections::VecDeque;

const RPC_CONCURRENCY_DEFAULT: usize = 4;
const MAX_REORG_DEPTH_DEFAULT: usize = 64;

/// A builder for streaming canonical block events from a historical block.
///
/// This wraps [`WatchBlocksFrom`] and performs reorg detection: when the chain tip changes
/// incompatibly, the stream yields [`CanonicalEvent::Removed`] for rolled-back blocks
/// followed by [`CanonicalEvent::Added`] for the new canonical chain segment.
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
                    // Because WatchBlocksFrom always emits sequential blocks in terms of number, 
                    // we can yield `Removed` events here.
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

#[derive(Debug)]
struct FixedBuf<T> {
    buf: VecDeque<T>,
}

impl<T> FixedBuf<T> {
    fn new(capacity: usize) -> Self {
        Self { buf: VecDeque::with_capacity(capacity.max(1)) }
    }

    /// Pushes `item` and discards the oldest item if the buffer is full.
    fn push(&mut self, item: T) {
        if self.buf.len() == self.buf.capacity() {
            self.buf.pop_front();
        }
        self.buf.push_back(item);
    }

    /// Returns the most recent item, if any.
    fn pop(&mut self) -> Option<T> {
        self.buf.pop_back()
    }

    fn last(&self) -> Option<&T> {
        self.buf.back()
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};
    use alloy_eips::BlockNumberOrTag;
    use alloy_primitives::{B256, U64};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::Block;
    use alloy_transport::{TransportError, TransportFut};
    use futures::StreamExt;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        task::Poll,
        time::Duration,
    };
    use tokio::time::timeout;

    // ── MockChain ──────────────────────────────────────────────────────

    struct ChainState {
        blocks: HashMap<u64, Block>,
        head: u64,
    }

    #[derive(Clone)]
    struct MockChain {
        state: Arc<RwLock<ChainState>>,
    }

    impl MockChain {
        fn new() -> Self {
            Self { state: Arc::new(RwLock::new(ChainState { blocks: HashMap::new(), head: 0 })) }
        }

        /// Insert blocks and set head to the highest block number.
        fn extend(&self, blocks: &[Block]) {
            let mut state = self.state.write().unwrap();
            for b in blocks {
                let number = b.header.inner.number;
                state.blocks.insert(number, b.clone());
                if number > state.head {
                    state.head = number;
                }
            }
        }

        /// Simulate a reorg: remove all blocks at height >= the first block's
        /// height, insert the new blocks, and set head to the highest.
        fn reorg(&self, blocks: &[Block]) {
            let mut state = self.state.write().unwrap();
            let min_height =
                blocks.iter().map(|b| b.header.inner.number).min().expect("reorg needs blocks");
            state.blocks.retain(|&h, _| h < min_height);
            let mut max = state.head;
            for b in blocks {
                let number = b.header.inner.number;
                state.blocks.insert(number, b.clone());
                if number > max {
                    max = number;
                }
            }
            state.head = max;
        }

        fn provider(&self) -> impl Provider {
            let transport = MockChainTransport { chain: self.clone() };
            ProviderBuilder::new().connect_client(RpcClient::new(transport, true))
        }

        fn handle_request(
            &self,
            req: &alloy_json_rpc::SerializedRequest,
        ) -> alloy_json_rpc::Response {
            let state = self.state.read().unwrap();
            let payload = match req.method() {
                "eth_blockNumber" => {
                    let raw = serde_json::to_string(&U64::from(state.head)).unwrap();
                    alloy_json_rpc::ResponsePayload::Success(
                        serde_json::value::RawValue::from_string(raw).unwrap(),
                    )
                }
                "eth_getBlockByNumber" => {
                    let params = req.params().expect("eth_getBlockByNumber requires params");
                    let (tag, _full): (BlockNumberOrTag, bool) =
                        serde_json::from_str(params.get()).unwrap();
                    let number = match tag {
                        BlockNumberOrTag::Number(n) => n,
                        BlockNumberOrTag::Latest => state.head,
                        _ => unimplemented!("unsupported block tag in MockChain: {tag:?}"),
                    };
                    let block = state.blocks.get(&number).cloned();
                    let raw = serde_json::to_string(&block).unwrap();
                    alloy_json_rpc::ResponsePayload::Success(
                        serde_json::value::RawValue::from_string(raw).unwrap(),
                    )
                }
                other => panic!("MockChain: unexpected RPC method `{other}`"),
            };
            alloy_json_rpc::Response { id: req.id().clone(), payload }
        }
    }

    #[derive(Clone)]
    struct MockChainTransport {
        chain: MockChain,
    }

    impl tower::Service<alloy_json_rpc::RequestPacket> for MockChainTransport {
        type Response = alloy_json_rpc::ResponsePacket;
        type Error = TransportError;
        type Future = TransportFut<'static>;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: alloy_json_rpc::RequestPacket) -> Self::Future {
            let chain = self.chain.clone();
            Box::pin(async move {
                Ok(match req {
                    alloy_json_rpc::RequestPacket::Single(req) => {
                        alloy_json_rpc::ResponsePacket::Single(chain.handle_request(&req))
                    }
                    alloy_json_rpc::RequestPacket::Batch(reqs) => {
                        alloy_json_rpc::ResponsePacket::Batch(
                            reqs.iter().map(|r| chain.handle_request(r)).collect(),
                        )
                    }
                })
            })
        }
    }

    fn block(number: u64, hash_last_byte: u8, parent_hash_last_byte: u8) -> Block {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block.header.hash = B256::with_last_byte(hash_last_byte);
        block.header.inner.parent_hash = B256::with_last_byte(parent_hash_last_byte);
        block
    }

    #[tokio::test]
    async fn emits_removed_then_added_on_reorg_within_buffer() {
        let chain = MockChain::new();
        // Initial chain: 1 -> 2 -> 3.
        chain.extend(&[block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]);

        let provider = chain.provider();
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

        // Reorg: replace block 3, add block 4.
        chain.reorg(&[block(3, 33, 2), block(4, 44, 33)]);

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
        let chain = MockChain::new();
        // Initial chain: 1 -> 2 -> 3.
        chain.extend(&[block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]);

        let provider = chain.provider();
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

        // Deep reorg: entirely new chain from height 2 onward.
        chain.reorg(&[block(2, 22, 11), block(3, 33, 22), block(4, 44, 33)]);

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
        let chain = MockChain::new();
        // Initial chain: 1 -> 2 -> 3 -> 4.
        chain.extend(&[block(1, 1, 0), block(2, 2, 1), block(3, 3, 2), block(4, 4, 3)]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        // Added 1, 2, 3, 4.
        for expected in [1_u64, 2, 3, 4] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        // Reorg: new chain from height 3 onward, adding block 5.
        chain.reorg(&[block(3, 33, 2), block(4, 44, 33), block(5, 5, 44)]);

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
        let chain = MockChain::new();
        // Initial chain: 1 -> 2 -> 3.
        chain.extend(&[block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]);

        let provider = chain.provider();
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

        // First reorg: block 4 expects parent hash 33, but block 3 has hash 34.
        // The stream will detect the mismatch during backfill and abandon reconciliation
        // via `continue 'stream`, then poll for new blocks.
        chain.reorg(&[block(3, 34, 2), block(4, 4, 33)]);

        // Removed(3) is yielded before the mismatch is discovered.
        let removed_3 =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        match removed_3 {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, 3);
                assert_eq!(block.header.hash, B256::with_last_byte(3));
            }
            other => panic!("expected Removed(3), got {other:?}"),
        }

        // Schedule the second reorg to happen while the stream is polling for new blocks.
        // The generator has already resumed and hit `continue 'stream` (because it saw
        // hash 34 instead of the expected 33). It's now waiting for the head to advance.
        let chain_clone = chain.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            chain_clone.reorg(&[block(3, 33, 2), block(4, 44, 33), block(5, 5, 44)]);
        });

        // Recovery: Added 3', Added 4', Added 5.
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
        let chain = MockChain::new();
        chain.extend(&[block(1, 1, 0)]);

        let provider = chain.provider();
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
        let chain = MockChain::new();
        let provider = chain.provider();
        let mut stream = provider.watch_canonical_blocks_from(0).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }
}
