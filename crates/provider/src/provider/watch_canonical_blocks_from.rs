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

#[derive(Debug)]
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

/// A stream of canonical block events produced by [`WatchCanonicalBlocksFrom`].
#[derive(Debug)]
#[pin_project]
pub struct WatchCanonicalBlocksFromStream<N: Network> {
    watch_blocks_from: WatchBlocksFrom<N>,
    #[pin]
    stream: Buffered<WatchBlocksFromStream<N>>,
    buffer: FixedBuf<N::BlockResponse>,
    state: WatchCanonicalBlocksFromState<N>,
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
pub(super) struct FixedBuf<T> {
    buf: VecDeque<T>,
}

impl<T> FixedBuf<T> {
    pub(super) fn new(capacity: usize) -> Self {
        Self { buf: VecDeque::with_capacity(capacity.max(1)) }
    }

    /// Pushes `item` and discards the oldest item if the buffer is full.
    pub(super) fn push(&mut self, item: T) {
        if self.buf.len() == self.buf.capacity() {
            self.buf.pop_front();
        }
        self.buf.push_back(item);
    }

    /// Returns the most recent item, if any.
    pub(super) fn pop(&mut self) -> Option<T> {
        self.buf.pop_back()
    }

    pub(super) fn last(&self) -> Option<&T> {
        self.buf.back()
    }

    pub(super) fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::watch_logs_test_utils::{block, MockChain},
        *,
    };
    use crate::Provider;
    use alloy_eips::BlockNumberOrTag;
    use alloy_primitives::B256;
    use alloy_rpc_types_eth::{Block, Log};
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    fn without_logs<const N: usize>(blocks: [Block; N]) -> Vec<(Block, Vec<Log>)> {
        blocks.into_iter().map(|block| (block, Vec::new())).collect()
    }

    async fn next_event(
        stream: &mut WatchCanonicalBlocksFromStream<alloy_network::Ethereum>,
    ) -> CanonicalEvent<Block> {
        timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap()
    }

    fn assert_added(event: CanonicalEvent<Block>, number: u64, hash_last_byte: u8) {
        match event {
            CanonicalEvent::Added(block) => {
                assert_eq!(block.header.number, number);
                assert_eq!(block.header.hash, B256::with_last_byte(hash_last_byte));
            }
            other => panic!("expected Added({number}), got {other:?}"),
        }
    }

    fn assert_removed(event: CanonicalEvent<Block>, number: u64, hash_last_byte: u8) {
        match event {
            CanonicalEvent::Removed(block) => {
                assert_eq!(block.header.number, number);
                assert_eq!(block.header.hash, B256::with_last_byte(hash_last_byte));
            }
            other => panic!("expected Removed({number}), got {other:?}"),
        }
    }

    fn canonical_stream(
        provider: &impl Provider,
        max_reorg_depth: usize,
    ) -> WatchCanonicalBlocksFromStream<alloy_network::Ethereum> {
        provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(max_reorg_depth)
            .into_stream()
    }

    #[tokio::test]
    async fn emits_removed_then_added_on_reorg_within_buffer() {
        let chain = MockChain::new();
        chain.extend(&without_logs([block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]));

        let provider = chain.provider();
        let mut stream = canonical_stream(&provider, 16);

        for number in [1, 2, 3] {
            assert_added(next_event(&mut stream).await, number, number as u8);
        }

        // Reorg: replace block 3, add block 4.
        chain.reorg(&without_logs([block(3, 33, 2), block(4, 44, 33)]));

        assert_removed(next_event(&mut stream).await, 3, 3);
        assert_added(next_event(&mut stream).await, 3, 33);
        assert_added(next_event(&mut stream).await, 4, 44);
    }

    #[tokio::test]
    async fn emits_error_when_reorg_exceeds_retained_history() {
        let chain = MockChain::new();
        chain.extend(&without_logs([block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]));

        let provider = chain.provider();
        let mut stream = canonical_stream(&provider, 2);

        for number in [1, 2, 3] {
            assert_added(next_event(&mut stream).await, number, number as u8);
        }

        // Deep reorg: entirely new chain from height 2 onward.
        chain.reorg(&without_logs([block(2, 22, 11), block(3, 33, 22), block(4, 44, 33)]));

        assert_removed(next_event(&mut stream).await, 3, 3);
        assert_removed(next_event(&mut stream).await, 2, 2);

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
        chain.extend(&without_logs([
            block(1, 1, 0),
            block(2, 2, 1),
            block(3, 3, 2),
            block(4, 4, 3),
        ]));

        let provider = chain.provider();
        let mut stream = canonical_stream(&provider, 8);

        for number in [1, 2, 3, 4] {
            assert_added(next_event(&mut stream).await, number, number as u8);
        }

        // Reorg: new chain from height 3 onward, adding block 5.
        chain.reorg(&without_logs([block(3, 33, 2), block(4, 44, 33), block(5, 5, 44)]));

        assert_removed(next_event(&mut stream).await, 4, 4);
        assert_removed(next_event(&mut stream).await, 3, 3);
        assert_added(next_event(&mut stream).await, 3, 33);
        assert_added(next_event(&mut stream).await, 4, 44);
        assert_added(next_event(&mut stream).await, 5, 5);
    }

    #[tokio::test]
    async fn recovers_when_chain_changes_during_backfill() {
        let chain = MockChain::new();
        chain.extend(&without_logs([block(1, 1, 0), block(2, 2, 1), block(3, 3, 2)]));

        let provider = chain.provider();
        let mut stream = canonical_stream(&provider, 8);

        for number in [1, 2, 3] {
            assert_added(next_event(&mut stream).await, number, number as u8);
        }

        // First reorg: block 4 expects parent hash 33, but block 3 has hash 34, so the stream
        // abandons reconciliation during backfill and polls for new blocks.
        chain.reorg(&without_logs([block(3, 34, 2), block(4, 4, 33)]));

        // Removed(3) is yielded before the mismatch is discovered.
        assert_removed(next_event(&mut stream).await, 3, 3);

        // Second reorg while the stream is waiting for the head to advance.
        let chain_clone = chain.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            chain_clone.reorg(&without_logs([block(3, 33, 2), block(4, 44, 33), block(5, 5, 44)]));
        });

        assert_added(next_event(&mut stream).await, 3, 33);
        assert_added(next_event(&mut stream).await, 4, 44);
        assert_added(next_event(&mut stream).await, 5, 5);
    }

    #[tokio::test]
    async fn clamps_zero_values_for_rpc_concurrency_and_reorg_depth() {
        let chain = MockChain::new();
        chain.extend(&without_logs([block(1, 1, 0)]));

        let provider = chain.provider();
        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(0)
            .max_reorg_depth(0)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1);
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
        // Intentionally inconsistent mock state to force a malformed backfill path:
        // request #1 -> block number 0 (hash=1), request #2 -> another block number 0
        // with a non-matching parent hash. This drives reconciliation to `height == 0`.
        chain.insert_at(1, block(0, 1, 0));
        chain.insert_at(2, block(0, 2, 9));

        let provider = chain.provider();
        let mut stream = canonical_stream(&provider, 8);

        assert_added(next_event(&mut stream).await, 0, 1);

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(format!("{err}").contains("genesis block"));
    }
}
