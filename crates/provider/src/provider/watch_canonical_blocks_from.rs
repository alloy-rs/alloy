use crate::{transport::TransportErrorKind, WatchBlocksFrom, WatchBlocksFromStream};
use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumberOrTag;
use alloy_network::{BlockResponse as _, Network};
use alloy_network_primitives::HeaderResponse;
use alloy_transport::{TransportError, TransportResult};
use futures::{stream::Buffered, Stream, StreamExt as _};
use pin_project::pin_project;
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

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

    /// Streams canonical blocks with full transaction bodies.
    pub fn full(mut self) -> Self {
        self.watch_blocks_from = self.watch_blocks_from.full();
        self
    }

    /// Streams canonical blocks with transaction hashes only.
    pub fn hashes(mut self) -> Self {
        self.watch_blocks_from = self.watch_blocks_from.hashes();
        self
    }

    /// Sets the poll interval used when the stream is caught up.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.watch_blocks_from = self.watch_blocks_from.poll_interval(poll_interval);
        self
    }

    /// Sets the head block tag used to determine stream progress.
    pub fn block_tag(mut self, block_tag: BlockNumberOrTag) -> Self {
        self.watch_blocks_from = self.watch_blocks_from.block_tag(block_tag);
        self
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
    pub fn into_stream(self) -> WatchCanonicalBlocksFromStream<N> {
        let Self { watch_blocks_from, rpc_concurrency, max_reorg_depth } = self;
        let stream = watch_blocks_from.clone().into_stream().buffered(rpc_concurrency.max(1));

        WatchCanonicalBlocksFromStream {
            watch_blocks_from,
            stream,
            buffer: FixedBuf::new(max_reorg_depth),
            state: WatchCanonicalBlocksFromState::PollNext,
        }
    }
}

enum WatchCanonicalBlocksFromState<N: Network> {
    /// Polling the next block from `watch_blocks_from(...).buffered(...)`.
    PollNext,
    /// Reconciling `next` with the canonical buffer by walking parents.
    Reconcile { next: N::BlockResponse, pending: VecDeque<N::BlockResponse> },
    /// Polling an in-flight parent fetch.
    FetchingParent {
        next: N::BlockResponse,
        pending: VecDeque<N::BlockResponse>,
        fut: super::BlockFut<N::BlockResponse>,
    },
    /// Emitting `Added` events for `pending`, then `next`.
    EmitPending { pending: VecDeque<N::BlockResponse>, next: Option<N::BlockResponse> },
    /// Yield one terminal error item and then end the stream.
    EmitError { err: TransportError },
    /// Stream terminated.
    Done,
}

impl<N: Network> WatchCanonicalBlocksFromState<N> {
    const fn name(&self) -> &'static str {
        match self {
            Self::PollNext => "PollNext",
            Self::Reconcile { .. } => "Reconcile",
            Self::FetchingParent { .. } => "FetchingParent",
            Self::EmitPending { .. } => "EmitPending",
            Self::EmitError { .. } => "EmitError",
            Self::Done => "Done",
        }
    }
}

/// A stream of canonical block events produced by [`WatchCanonicalBlocksFrom`].
#[pin_project]
pub struct WatchCanonicalBlocksFromStream<N: Network> {
    watch_blocks_from: WatchBlocksFrom<N>,
    #[pin]
    stream: Buffered<WatchBlocksFromStream<N>>,
    buffer: FixedBuf<N::BlockResponse>,
    state: WatchCanonicalBlocksFromState<N>,
}

impl<N: Network> std::fmt::Debug for WatchCanonicalBlocksFromStream<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchCanonicalBlocksFromStream")
            .field("state", &self.state.name())
            .finish_non_exhaustive()
    }
}

impl<N: Network> Stream for WatchCanonicalBlocksFromStream<N> {
    type Item = TransportResult<CanonicalEvent<N::BlockResponse>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let state = std::mem::replace(this.state, WatchCanonicalBlocksFromState::Done);
            match state {
                WatchCanonicalBlocksFromState::PollNext => match this.stream.as_mut().poll_next(cx)
                {
                    Poll::Pending => {
                        *this.state = WatchCanonicalBlocksFromState::PollNext;
                        return Poll::Pending;
                    }
                    Poll::Ready(None) => {
                        *this.state = WatchCanonicalBlocksFromState::Done;
                    }
                    Poll::Ready(Some(Ok(next))) => {
                        *this.state = WatchCanonicalBlocksFromState::Reconcile {
                            next,
                            pending: VecDeque::new(),
                        };
                    }
                    Poll::Ready(Some(Err(err))) => {
                        *this.state = WatchCanonicalBlocksFromState::EmitError { err };
                    }
                },
                WatchCanonicalBlocksFromState::Reconcile { next, pending } => {
                    let front = pending.front().unwrap_or(&next);
                    let Some(canonical_tip) = this.buffer.last() else {
                        *this.state = WatchCanonicalBlocksFromState::EmitPending {
                            pending,
                            next: Some(next),
                        };
                        continue;
                    };

                    let parent_hash = front.header().parent_hash();
                    if parent_hash == canonical_tip.header().hash() {
                        *this.state = WatchCanonicalBlocksFromState::EmitPending {
                            pending,
                            next: Some(next),
                        };
                        continue;
                    }

                    // Reorg detected: `front` does not build on canonical tip.
                    // Because WatchBlocksFrom emits strictly sequential heights, we can
                    // remove the tip when heights are adjacent.
                    let height = front.header().number();
                    let canonical_height = canonical_tip.header().number();
                    if canonical_height + 1 == height {
                        let removed = this
                            .buffer
                            .pop()
                            .expect("position is always < canonical buffer length");
                        if this.buffer.len() == 0 {
                            *this.state = WatchCanonicalBlocksFromState::EmitError {
                                err: TransportErrorKind::custom_str(
                                    "Deep reorg detected; no canonical history retained.",
                                ),
                            };
                        } else {
                            *this.state =
                                WatchCanonicalBlocksFromState::Reconcile { next, pending };
                        }
                        return Poll::Ready(Some(Ok(CanonicalEvent::Removed(removed))));
                    }

                    let Some(parent_height) = height.checked_sub(1) else {
                        *this.state = WatchCanonicalBlocksFromState::EmitError {
                            err: TransportErrorKind::custom_str(
                                "Cannot backfill parent for genesis block during canonical reconciliation.",
                            ),
                        };
                        continue;
                    };

                    let watch_blocks_from = this.watch_blocks_from.clone();
                    let fut = watch_blocks_from.get_block(parent_height);
                    *this.state =
                        WatchCanonicalBlocksFromState::FetchingParent { next, pending, fut };
                }
                WatchCanonicalBlocksFromState::FetchingParent { next, mut pending, mut fut } => {
                    match Pin::new(&mut fut).poll(cx) {
                        Poll::Pending => {
                            *this.state = WatchCanonicalBlocksFromState::FetchingParent {
                                next,
                                pending,
                                fut,
                            };
                            return Poll::Pending;
                        }
                        Poll::Ready(Err(err)) => {
                            *this.state = WatchCanonicalBlocksFromState::EmitError { err };
                        }
                        Poll::Ready(Ok(parent)) => {
                            let front = pending.front().unwrap_or(&next);
                            if parent.header().hash() != front.header().parent_hash() {
                                // Parent no longer matches: a second reorg happened while
                                // reconciling. Abandon this item and continue with next blocks.
                                *this.state = WatchCanonicalBlocksFromState::PollNext;
                                continue;
                            }

                            pending.push_front(parent);
                            *this.state =
                                WatchCanonicalBlocksFromState::Reconcile { next, pending };
                        }
                    }
                }
                WatchCanonicalBlocksFromState::EmitPending { mut pending, mut next } => {
                    if let Some(block) = pending.pop_front() {
                        this.buffer.push(block.clone());
                        *this.state = WatchCanonicalBlocksFromState::EmitPending { pending, next };
                        return Poll::Ready(Some(Ok(CanonicalEvent::Added(block))));
                    }

                    if let Some(next) = next.take() {
                        this.buffer.push(next.clone());
                        *this.state = WatchCanonicalBlocksFromState::PollNext;
                        return Poll::Ready(Some(Ok(CanonicalEvent::Added(next))));
                    }

                    *this.state = WatchCanonicalBlocksFromState::PollNext;
                }
                WatchCanonicalBlocksFromState::EmitError { err } => {
                    *this.state = WatchCanonicalBlocksFromState::Done;
                    return Poll::Ready(Some(Err(err)));
                }
                WatchCanonicalBlocksFromState::Done => {
                    *this.state = WatchCanonicalBlocksFromState::Done;
                    return Poll::Ready(None);
                }
            }
        }
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
    async fn canonical_builder_exposes_watch_blocks_from_methods() {
        let chain = MockChain::new();
        chain.extend(&[block(1, 1, 0)]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .hashes()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
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

    #[tokio::test]
    async fn errors_instead_of_underflow_when_backfilling_genesis_parent() {
        let chain = MockChain::new();
        {
            let mut state = chain.state.write().unwrap();
            state.head = 2;
            // Intentionally inconsistent mock state to force a malformed backfill path:
            // request #1 -> block number 0 (hash=1), request #2 -> another block number 0
            // with a non-matching parent hash. This drives reconciliation to `height == 0`.
            state.blocks.insert(1, block(0, 1, 0));
            state.blocks.insert(2, block(0, 2, 9));
        }

        let provider = chain.provider();
        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        match first {
            CanonicalEvent::Added(block) => assert_eq!(block.header.number, 0),
            other => panic!("expected Added(0), got {other:?}"),
        }

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(format!("{err}").contains("genesis block"));
    }
}
