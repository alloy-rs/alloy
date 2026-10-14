use super::{
    watch_canonical_blocks_from::FixedBuf, BlockLogs, BlockLogsFut, CanonicalEvent, WatchLogsFrom,
    WatchLogsFromStream,
};
use crate::transport::TransportErrorKind;
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

/// A builder for streaming canonical log batches from a historical block.
///
/// This wraps a block/log source stream and performs reorg detection like
/// [`super::WatchCanonicalBlocksFrom`]. Each item represents one canonical block and the logs in
/// that block matching the configured [`Filter`](alloy_rpc_types_eth::Filter). When the chain tip
/// changes incompatibly, the stream yields [`CanonicalEvent::Removed`] for retained block log
/// batches followed by
/// [`CanonicalEvent::Added`] for the new canonical chain segment.
///
/// The source stream fetches each block and a one-block log range concurrently, falling back to an
/// exact block-hash log query when the range result is empty or ambiguous. This keeps each emitted
/// batch internally consistent with the block it carries. If a later block reveals that an emitted
/// batch was on a fork, the stream uses retained history to emit [`CanonicalEvent::Removed`] for
/// that batch before adding the new canonical segment.
///
/// Blocks with no matching logs are still emitted with an empty log vector. This keeps stream
/// progress and [`max_reorg_depth`](Self::max_reorg_depth) aligned to block depth rather than log
/// count.
///
/// RPC errors are surfaced to the caller. Configure retries on the underlying client transport for
/// transport-level retry behavior.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchCanonicalLogsFrom<N: Network> {
    watch_logs_from: WatchLogsFrom<N>,
    rpc_concurrency: usize,
    max_reorg_depth: usize,
}

impl<N: Network> WatchCanonicalLogsFrom<N> {
    pub(crate) const fn new(watch_logs_from: WatchLogsFrom<N>) -> Self {
        Self {
            watch_logs_from,
            rpc_concurrency: RPC_CONCURRENCY_DEFAULT,
            max_reorg_depth: MAX_REORG_DEPTH_DEFAULT,
        }
    }

    /// Streams block log batches with full transaction bodies in the block response.
    pub fn full(mut self) -> Self {
        self.watch_logs_from = self.watch_logs_from.full();
        self
    }

    /// Streams block log batches with transaction hashes only in the block response.
    pub fn hashes(mut self) -> Self {
        self.watch_logs_from = self.watch_logs_from.hashes();
        self
    }

    /// Sets the poll interval used when the stream is caught up to the configured head tag.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.watch_logs_from = self.watch_logs_from.poll_interval(poll_interval);
        self
    }

    /// Sets the head block tag used to determine stream progress.
    ///
    /// The stream fetches all block log batches from `start_block` through this head, then polls
    /// again after [`poll_interval`](Self::poll_interval) once caught up.
    pub fn block_tag(mut self, block_tag: BlockNumberOrTag) -> Self {
        self.watch_logs_from = self.watch_logs_from.block_tag(block_tag);
        self
    }

    /// Sets the number of in-flight block/log RPC request groups.
    ///
    /// Results are still emitted in block-number order. A value of `0` is clamped to `1`.
    pub const fn rpc_concurrency(mut self, rpc_concurrency: usize) -> Self {
        self.rpc_concurrency = if rpc_concurrency == 0 { 1 } else { rpc_concurrency };
        self
    }

    /// Sets the maximum number of canonical blocks and their logs retained for reorg detection.
    ///
    /// Removed events can only be emitted for blocks still retained in this buffer. A reorg deeper
    /// than the retained history yields a terminal error. A value of `0` is clamped to `1`.
    pub const fn max_reorg_depth(mut self, max_reorg_depth: usize) -> Self {
        self.max_reorg_depth = if max_reorg_depth == 0 { 1 } else { max_reorg_depth };
        self
    }

    /// Converts the builder into a stream of canonical block log events.
    ///
    /// The returned stream emits [`CanonicalEvent`] values containing [`BlockLogs`].
    /// Added events contain logs with `removed = false`; removed events contain retained logs with
    /// `removed = true`.
    pub fn into_stream(self) -> WatchCanonicalLogsFromStream<N> {
        let Self { watch_logs_from, rpc_concurrency, max_reorg_depth } = self;
        let stream = watch_logs_from.clone().into_stream().buffered(rpc_concurrency.max(1));

        WatchCanonicalLogsFromStream {
            watch_logs_from,
            stream,
            buffer: FixedBuf::new(max_reorg_depth),
            state: WatchCanonicalLogsFromState::PollNext,
        }
    }
}

#[derive(Debug)]
enum WatchCanonicalLogsFromState<N: Network> {
    /// Polling the next block/log pair from `WatchLogsFromStream`.
    PollNext,
    /// Reconciling `next` with the canonical buffer by walking parents.
    Reconcile { next: BlockLogs<N>, pending: VecDeque<BlockLogs<N>> },
    /// Polling an in-flight parent block/log fetch.
    FetchingParent { next: BlockLogs<N>, pending: VecDeque<BlockLogs<N>>, fut: BlockLogsFut<N> },
    /// Emitting block log batches for `pending`, then `next`.
    EmitPending { pending: VecDeque<BlockLogs<N>>, next: Option<BlockLogs<N>> },
    /// Yield one terminal error item and then end the stream.
    EmitError { err: TransportError },
    /// Stream terminated.
    Done,
}

/// A stream of canonical log batches produced by [`WatchCanonicalLogsFrom`].
///
/// The stream emits one item per block, wrapped in [`CanonicalEvent`]. Added batches contain logs
/// proven to match their block hash. Removed batches are served from retained history so the stream
/// does not need to query logs for rolled-back blocks after a reorg.
#[derive(Debug)]
#[pin_project]
pub struct WatchCanonicalLogsFromStream<N: Network> {
    watch_logs_from: WatchLogsFrom<N>,
    #[pin]
    stream: Buffered<WatchLogsFromStream<N>>,
    buffer: FixedBuf<BlockLogs<N>>,
    state: WatchCanonicalLogsFromState<N>,
}

impl<N: Network> Stream for WatchCanonicalLogsFromStream<N> {
    type Item = TransportResult<CanonicalEvent<BlockLogs<N>>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let state = std::mem::replace(this.state, WatchCanonicalLogsFromState::Done);
            match state {
                WatchCanonicalLogsFromState::PollNext => match this.stream.as_mut().poll_next(cx) {
                    Poll::Pending => {
                        *this.state = WatchCanonicalLogsFromState::PollNext;
                        return Poll::Pending;
                    }
                    Poll::Ready(None) => {
                        *this.state = WatchCanonicalLogsFromState::Done;
                    }
                    Poll::Ready(Some(Ok(next))) => {
                        *this.state = WatchCanonicalLogsFromState::Reconcile {
                            next,
                            pending: VecDeque::new(),
                        };
                    }
                    Poll::Ready(Some(Err(err))) => {
                        *this.state = WatchCanonicalLogsFromState::EmitError { err };
                    }
                },
                WatchCanonicalLogsFromState::Reconcile { next, pending } => {
                    let front = pending.front().unwrap_or(&next);
                    let Some(canonical_tip) = this.buffer.last() else {
                        *this.state =
                            WatchCanonicalLogsFromState::EmitPending { pending, next: Some(next) };
                        continue;
                    };

                    let parent_hash = front.block.header().parent_hash();
                    if parent_hash == canonical_tip.block.header().hash() {
                        *this.state =
                            WatchCanonicalLogsFromState::EmitPending { pending, next: Some(next) };
                        continue;
                    }

                    let height = front.block.header().number();
                    let canonical_height = canonical_tip.block.header().number();
                    if canonical_height + 1 == height {
                        let removed = this
                            .buffer
                            .pop()
                            .expect("position is always < canonical buffer length");
                        let after = if this.buffer.len() == 0 {
                            WatchCanonicalLogsFromState::EmitError {
                                err: TransportErrorKind::custom_str(
                                    "Deep reorg detected; no canonical log history retained.",
                                ),
                            }
                        } else {
                            WatchCanonicalLogsFromState::Reconcile { next, pending }
                        };
                        *this.state = after;
                        return Poll::Ready(Some(Ok(block_log_event(removed, true))));
                    }

                    let Some(parent_height) = height.checked_sub(1) else {
                        *this.state = WatchCanonicalLogsFromState::EmitError {
                            err: TransportErrorKind::custom_str(
                                "Cannot backfill parent for genesis block during canonical log reconciliation.",
                            ),
                        };
                        continue;
                    };

                    let watch_logs_from = this.watch_logs_from.clone();
                    let fut = watch_logs_from.get_block_logs(parent_height);
                    *this.state =
                        WatchCanonicalLogsFromState::FetchingParent { next, pending, fut };
                }
                WatchCanonicalLogsFromState::FetchingParent { next, mut pending, mut fut } => {
                    match Pin::new(&mut fut).poll(cx) {
                        Poll::Pending => {
                            *this.state =
                                WatchCanonicalLogsFromState::FetchingParent { next, pending, fut };
                            return Poll::Pending;
                        }
                        Poll::Ready(Err(err)) => {
                            *this.state = WatchCanonicalLogsFromState::EmitError { err };
                        }
                        Poll::Ready(Ok(parent)) => {
                            let front = pending.front().unwrap_or(&next);
                            if parent.block.header().hash() != front.block.header().parent_hash() {
                                // Parent no longer matches: a second reorg happened while
                                // reconciling. Abandon this item and continue with next blocks.
                                *this.state = WatchCanonicalLogsFromState::PollNext;
                                continue;
                            }

                            pending.push_front(parent);
                            *this.state = WatchCanonicalLogsFromState::Reconcile { next, pending };
                        }
                    }
                }
                WatchCanonicalLogsFromState::EmitPending { mut pending, mut next } => {
                    if let Some(block_logs) = pending.pop_front() {
                        this.buffer.push(block_logs.clone());
                        *this.state = WatchCanonicalLogsFromState::EmitPending { pending, next };
                        return Poll::Ready(Some(Ok(block_log_event(block_logs, false))));
                    }

                    if let Some(block_logs) = next.take() {
                        this.buffer.push(block_logs.clone());
                        *this.state = WatchCanonicalLogsFromState::PollNext;
                        return Poll::Ready(Some(Ok(block_log_event(block_logs, false))));
                    }

                    *this.state = WatchCanonicalLogsFromState::PollNext;
                }
                WatchCanonicalLogsFromState::EmitError { err } => {
                    *this.state = WatchCanonicalLogsFromState::Done;
                    return Poll::Ready(Some(Err(err)));
                }
                WatchCanonicalLogsFromState::Done => {
                    *this.state = WatchCanonicalLogsFromState::Done;
                    return Poll::Ready(None);
                }
            }
        }
    }
}

fn block_log_event<N: Network>(
    mut block_logs: BlockLogs<N>,
    removed: bool,
) -> CanonicalEvent<BlockLogs<N>> {
    for log in &mut block_logs.logs {
        log.removed = removed;
    }

    if removed {
        CanonicalEvent::Removed(block_logs)
    } else {
        CanonicalEvent::Added(block_logs)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::watch_logs_test_utils::{assert_batch, block, log, MockChain},
        *,
    };
    use crate::Provider;
    use alloy_eips::BlockNumberOrTag;
    use alloy_rpc_types_eth::Filter;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    async fn next_event(
        stream: &mut WatchCanonicalLogsFromStream<alloy_network::Ethereum>,
    ) -> CanonicalEvent<BlockLogs<alloy_network::Ethereum>> {
        timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap()
    }

    fn assert_added(
        event: CanonicalEvent<BlockLogs<alloy_network::Ethereum>>,
        number: u64,
        hash_last_byte: u8,
        log_count: usize,
    ) {
        match event {
            CanonicalEvent::Added(block_logs) => {
                assert_batch(&block_logs, number, hash_last_byte, false, log_count);
            }
            other => panic!("expected Added({number}), got {other:?}"),
        }
    }

    fn assert_removed(
        event: CanonicalEvent<BlockLogs<alloy_network::Ethereum>>,
        number: u64,
        hash_last_byte: u8,
        log_count: usize,
    ) {
        match event {
            CanonicalEvent::Removed(block_logs) => {
                assert_batch(&block_logs, number, hash_last_byte, true, log_count);
            }
            other => panic!("expected Removed({number}), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn emits_removed_then_added_logs_on_reorg_within_buffer() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)]), (block(2, 2, 1), vec![log(2, 2, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(2)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_added(next_event(&mut stream).await, 2, 2, 1);

        chain.reorg(&[
            (block(2, 22, 1), vec![log(2, 22, 0)]),
            (block(3, 33, 22), vec![log(3, 33, 0)]),
        ]);

        assert_removed(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 2, 22, 1);
        assert_added(next_event(&mut stream).await, 3, 33, 1);
    }

    #[tokio::test]
    async fn emits_empty_log_batches_for_canonical_progress() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![]), (block(2, 2, 1), vec![log(2, 2, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(2)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 0);
        assert_added(next_event(&mut stream).await, 2, 2, 1);
    }

    #[tokio::test]
    async fn backfills_parent_log_batches_when_reorg_ancestor_is_retained() {
        let chain = MockChain::new();
        chain.extend(&[
            (block(1, 1, 0), vec![log(1, 1, 0)]),
            (block(2, 2, 1), vec![log(2, 2, 0)]),
            (block(3, 3, 2), vec![log(3, 3, 0)]),
            (block(4, 4, 3), vec![log(4, 4, 0)]),
        ]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_added(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 3, 3, 1);
        assert_added(next_event(&mut stream).await, 4, 4, 1);

        chain.reorg(&[
            (block(3, 33, 2), vec![log(3, 33, 0)]),
            (block(4, 44, 33), vec![log(4, 44, 0)]),
            (block(5, 5, 44), vec![log(5, 5, 0)]),
        ]);

        assert_removed(next_event(&mut stream).await, 4, 4, 1);
        assert_removed(next_event(&mut stream).await, 3, 3, 1);
        assert_added(next_event(&mut stream).await, 3, 33, 1);
        assert_added(next_event(&mut stream).await, 4, 44, 1);
        assert_added(next_event(&mut stream).await, 5, 5, 1);
    }

    #[tokio::test]
    async fn recovers_when_chain_changes_during_log_backfill() {
        let chain = MockChain::new();
        chain.extend(&[
            (block(1, 1, 0), vec![log(1, 1, 0)]),
            (block(2, 2, 1), vec![log(2, 2, 0)]),
            (block(3, 3, 2), vec![log(3, 3, 0)]),
        ]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_added(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 3, 3, 1);

        chain.reorg(&[
            (block(3, 34, 2), vec![log(3, 34, 0)]),
            (block(4, 4, 33), vec![log(4, 4, 0)]),
        ]);

        assert_removed(next_event(&mut stream).await, 3, 3, 1);

        let chain_clone = chain.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            chain_clone.reorg(&[
                (block(3, 33, 2), vec![log(3, 33, 0)]),
                (block(4, 44, 33), vec![log(4, 44, 0)]),
                (block(5, 5, 44), vec![log(5, 5, 0)]),
            ]);
        });

        assert_added(next_event(&mut stream).await, 3, 33, 1);
        assert_added(next_event(&mut stream).await, 4, 44, 1);
        assert_added(next_event(&mut stream).await, 5, 5, 1);
    }

    #[tokio::test]
    async fn clamps_zero_values_for_rpc_concurrency_and_reorg_depth() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(0)
            .max_reorg_depth(0)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
    }

    #[tokio::test]
    async fn canonical_builder_exposes_watch_logs_from_methods() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(1))
            .poll_interval(Duration::from_millis(1))
            .full()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_eq!(chain.block_request_full_flags(), vec![true]);
    }

    #[tokio::test]
    async fn watch_logs_from_canonical_matches_provider_method() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(1))
            .poll_interval(Duration::from_millis(1))
            .hashes()
            .canonical()
            .rpc_concurrency(1)
            .max_reorg_depth(8)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_eq!(chain.block_request_full_flags(), vec![false]);
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let chain = MockChain::new();
        let provider = chain.provider();
        let mut stream = provider.watch_canonical_logs_from(0, &Filter::new()).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn emits_removed_logs_before_deep_reorg_error() {
        let chain = MockChain::new();
        chain.extend(&[
            (block(1, 1, 0), vec![log(1, 1, 0)]),
            (block(2, 2, 1), vec![log(2, 2, 0)]),
            (block(3, 3, 2), vec![log(3, 3, 0)]),
        ]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_canonical_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .rpc_concurrency(2)
            .max_reorg_depth(2)
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_added(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 3, 3, 1);

        chain.reorg(&[
            (block(2, 22, 11), vec![log(2, 22, 0)]),
            (block(3, 33, 22), vec![log(3, 33, 0)]),
            (block(4, 44, 33), vec![log(4, 44, 0)]),
        ]);

        assert_removed(next_event(&mut stream).await, 3, 3, 1);
        assert_removed(next_event(&mut stream).await, 2, 2, 1);

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(format!("{err}").contains("Deep reorg detected"));
    }
}
