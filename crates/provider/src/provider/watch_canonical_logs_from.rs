use super::{
    BlockLogs, BlockLogsFut, CanonicalEvent, CanonicalStore, InMemoryCanonicalStore, WatchLogsFrom,
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
    fmt,
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
pub struct WatchCanonicalLogsFrom<N, S = InMemoryCanonicalStore<BlockLogs<N>>>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
    watch_logs_from: WatchLogsFrom<N>,
    rpc_concurrency: usize,
    block_store: S,
}

impl<N: Network> WatchCanonicalLogsFrom<N> {
    pub(crate) fn new(watch_logs_from: WatchLogsFrom<N>) -> Self {
        Self {
            watch_logs_from,
            rpc_concurrency: RPC_CONCURRENCY_DEFAULT,
            block_store: InMemoryCanonicalStore::<BlockLogs<N>>::new(MAX_REORG_DEPTH_DEFAULT),
        }
    }
}

impl<N, S> WatchCanonicalLogsFrom<N, S>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
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

    /// Sets the store used to fetch previously emitted canonical block log batches during reorg
    /// handling.
    pub fn block_store<S2>(self, block_store: S2) -> WatchCanonicalLogsFrom<N, S2>
    where
        S2: CanonicalStore<BlockLogs<N>>,
    {
        WatchCanonicalLogsFrom {
            watch_logs_from: self.watch_logs_from,
            rpc_concurrency: self.rpc_concurrency,
            block_store,
        }
    }

    /// Converts the builder into a stream of canonical block log events.
    ///
    /// The returned stream emits [`CanonicalEvent`] values containing [`BlockLogs`].
    /// Added events contain logs with `removed = false`; removed events contain retained logs with
    /// `removed = true`.
    pub fn into_stream(self) -> WatchCanonicalLogsFromStream<N, S> {
        let Self { watch_logs_from, rpc_concurrency, block_store } = self;
        let stream = watch_logs_from.clone().into_stream().buffered(rpc_concurrency.max(1));

        WatchCanonicalLogsFromStream {
            watch_logs_from,
            stream,
            block_store,
            canonical_tip: None,
            state: WatchCanonicalLogsFromState::PollNext,
        }
    }
}

impl<N: Network> WatchCanonicalLogsFrom<N, InMemoryCanonicalStore<BlockLogs<N>>> {
    /// Sets the maximum number of canonical blocks and their logs retained by the default
    /// in-memory store.
    pub fn max_reorg_depth(mut self, max_reorg_depth: usize) -> Self {
        self.block_store = InMemoryCanonicalStore::<BlockLogs<N>>::new(max_reorg_depth);
        self
    }
}

#[pin_project(
    project = WatchCanonicalLogsFromStateProj,
    project_replace = WatchCanonicalLogsFromStateProjOwn,
)]
enum WatchCanonicalLogsFromState<N, S>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
    /// Polling the next block/log pair from `WatchLogsFromStream`.
    PollNext,
    /// Reconciling `next` with the canonical buffer by walking parents.
    Reconcile { next: BlockLogs<N>, pending: VecDeque<BlockLogs<N>> },
    /// Polling an in-flight parent block/log fetch.
    FetchingParent {
        next: BlockLogs<N>,
        pending: VecDeque<BlockLogs<N>>,
        #[pin]
        fut: BlockLogsFut<N>,
    },
    /// Polling an in-flight fetch for the rolled-back canonical block logs.
    FetchingRemoved {
        next: BlockLogs<N>,
        pending: VecDeque<BlockLogs<N>>,
        block_number: u64,
        #[pin]
        fut: S::PopFuture,
    },
    /// Polling an in-flight fetch for the new canonical tip after one rollback.
    FetchingTipAfterRemoved {
        next: BlockLogs<N>,
        pending: VecDeque<BlockLogs<N>>,
        removed: BlockLogs<N>,
        #[pin]
        fut: S::GetFuture,
    },
    /// Emitting block log batches for `pending`, then `next`.
    EmitPending { pending: VecDeque<BlockLogs<N>>, next: Option<BlockLogs<N>> },
    /// Polling an in-flight store insert before emitting an `Added` event.
    StoringAdded {
        pending: VecDeque<BlockLogs<N>>,
        next: Option<BlockLogs<N>>,
        block_logs: BlockLogs<N>,
        #[pin]
        fut: S::PushFuture,
    },
    /// Yield one terminal error item and then end the stream.
    EmitError { err: TransportError },
    /// Stream terminated.
    Done,
}

impl<N, S> fmt::Debug for WatchCanonicalLogsFromState<N, S>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PollNext => f.write_str("PollNext"),
            Self::Reconcile { next, pending } => {
                f.debug_struct("Reconcile").field("next", next).field("pending", pending).finish()
            }
            Self::FetchingParent { next, pending, .. } => f
                .debug_struct("FetchingParent")
                .field("next", next)
                .field("pending", pending)
                .finish_non_exhaustive(),
            Self::FetchingRemoved { next, pending, block_number, .. } => f
                .debug_struct("FetchingRemoved")
                .field("next", next)
                .field("pending", pending)
                .field("block_number", block_number)
                .finish_non_exhaustive(),
            Self::FetchingTipAfterRemoved { next, pending, removed, .. } => f
                .debug_struct("FetchingTipAfterRemoved")
                .field("next", next)
                .field("pending", pending)
                .field("removed", removed)
                .finish_non_exhaustive(),
            Self::EmitPending { pending, next } => {
                f.debug_struct("EmitPending").field("pending", pending).field("next", next).finish()
            }
            Self::StoringAdded { pending, next, block_logs, .. } => f
                .debug_struct("StoringAdded")
                .field("pending", pending)
                .field("next", next)
                .field("block_logs", block_logs)
                .finish_non_exhaustive(),
            Self::EmitError { err } => f.debug_struct("EmitError").field("err", err).finish(),
            Self::Done => f.write_str("Done"),
        }
    }
}

/// A stream of canonical log batches produced by [`WatchCanonicalLogsFrom`].
///
/// The stream emits one item per block, wrapped in [`CanonicalEvent`]. Added batches contain logs
/// proven to match their block hash. Removed batches are served from retained history so the stream
/// does not need to query logs for rolled-back blocks after a reorg.
#[derive(Debug)]
#[pin_project]
pub struct WatchCanonicalLogsFromStream<N, S = InMemoryCanonicalStore<BlockLogs<N>>>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
    watch_logs_from: WatchLogsFrom<N>,
    #[pin]
    stream: Buffered<WatchLogsFromStream<N>>,
    block_store: S,
    canonical_tip: Option<N::HeaderResponse>,
    #[pin]
    state: WatchCanonicalLogsFromState<N, S>,
}

impl<N, S> Stream for WatchCanonicalLogsFromStream<N, S>
where
    N: Network,
    S: CanonicalStore<BlockLogs<N>>,
{
    type Item = TransportResult<CanonicalEvent<BlockLogs<N>>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().project() {
                WatchCanonicalLogsFromStateProj::Done => return Poll::Ready(None),
                WatchCanonicalLogsFromStateProj::PollNext => {
                    match this.stream.as_mut().poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => {
                            this.state.set(WatchCanonicalLogsFromState::Done);
                        }
                        Poll::Ready(Some(Ok(next))) => {
                            this.state.set(WatchCanonicalLogsFromState::Reconcile {
                                next,
                                pending: VecDeque::new(),
                            });
                        }
                        Poll::Ready(Some(Err(err))) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError { err });
                        }
                    }
                }
                WatchCanonicalLogsFromStateProj::Reconcile { .. } => {
                    let WatchCanonicalLogsFromStateProjOwn::Reconcile { next, pending } =
                        this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };

                    let front = pending.front().unwrap_or(&next);
                    let Some(canonical_tip) = this.canonical_tip.as_ref() else {
                        this.state.set(WatchCanonicalLogsFromState::EmitPending {
                            pending,
                            next: Some(next),
                        });
                        continue;
                    };

                    let parent_hash = front.header().parent_hash();
                    if parent_hash == canonical_tip.hash() {
                        this.state.set(WatchCanonicalLogsFromState::EmitPending {
                            pending,
                            next: Some(next),
                        });
                        continue;
                    }

                    let height = front.header().number();
                    let canonical_height = canonical_tip.number();
                    if canonical_height + 1 == height {
                        let fut = this.block_store.pop();
                        this.state.set(WatchCanonicalLogsFromState::FetchingRemoved {
                            next,
                            pending,
                            block_number: canonical_height,
                            fut,
                        });
                        continue;
                    }

                    let Some(parent_height) = height.checked_sub(1) else {
                        this.state.set(WatchCanonicalLogsFromState::EmitError {
                            err: TransportErrorKind::custom_str(
                                "Cannot backfill parent for genesis block during canonical log reconciliation.",
                            ),
                        });
                        continue;
                    };

                    let watch_logs_from = this.watch_logs_from.clone();
                    let fut = watch_logs_from.get_block_logs(parent_height);
                    this.state.set(WatchCanonicalLogsFromState::FetchingParent {
                        next,
                        pending,
                        fut,
                    });
                }
                WatchCanonicalLogsFromStateProj::FetchingParent { fut, .. } => {
                    let parent = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError { err });
                            continue;
                        }
                        Poll::Ready(Ok(parent)) => parent,
                    };
                    let WatchCanonicalLogsFromStateProjOwn::FetchingParent {
                        next,
                        mut pending,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };
                    let front = pending.front().unwrap_or(&next);
                    if parent.header().hash() != front.header().parent_hash() {
                        // Parent no longer matches: a second reorg happened while
                        // reconciling. Abandon this item and continue with next blocks.
                        this.state.set(WatchCanonicalLogsFromState::PollNext);
                        continue;
                    }

                    pending.push_front(parent);
                    this.state.set(WatchCanonicalLogsFromState::Reconcile { next, pending });
                }
                WatchCanonicalLogsFromStateProj::FetchingRemoved { fut, .. } => {
                    let removed = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(None)) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError {
                                err: TransportErrorKind::custom_str(
                                    "Canonical log history is missing an expected block.",
                                ),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(Some(removed))) => removed,
                    };
                    let WatchCanonicalLogsFromStateProjOwn::FetchingRemoved {
                        next,
                        pending,
                        block_number,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };
                    let Some(parent_number) = block_number.checked_sub(1) else {
                        *this.canonical_tip = None;
                        this.state.set(WatchCanonicalLogsFromState::EmitError {
                            err: deep_reorg_log_error(),
                        });
                        return Poll::Ready(Some(Ok(block_log_event(removed, true))));
                    };
                    let fut = this.block_store.get(parent_number);
                    this.state.set(WatchCanonicalLogsFromState::FetchingTipAfterRemoved {
                        next,
                        pending,
                        removed,
                        fut,
                    });
                }
                WatchCanonicalLogsFromStateProj::FetchingTipAfterRemoved { fut, .. } => {
                    let previous_tip = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(previous_tip)) => previous_tip,
                    };
                    let WatchCanonicalLogsFromStateProjOwn::FetchingTipAfterRemoved {
                        next,
                        pending,
                        removed,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };
                    *this.canonical_tip =
                        previous_tip.as_ref().map(|block_logs| block_logs.header().clone());
                    if this.canonical_tip.is_some() {
                        this.state.set(WatchCanonicalLogsFromState::Reconcile { next, pending });
                    } else {
                        this.state.set(WatchCanonicalLogsFromState::EmitError {
                            err: deep_reorg_log_error(),
                        });
                    }
                    return Poll::Ready(Some(Ok(block_log_event(removed, true))));
                }
                WatchCanonicalLogsFromStateProj::EmitPending { .. } => {
                    let WatchCanonicalLogsFromStateProjOwn::EmitPending { mut pending, mut next } =
                        this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };

                    if let Some(block_logs) = pending.pop_front() {
                        let fut = this.block_store.push(block_logs.clone());
                        this.state.set(WatchCanonicalLogsFromState::StoringAdded {
                            pending,
                            next,
                            block_logs,
                            fut,
                        });
                        continue;
                    }

                    if let Some(block_logs) = next.take() {
                        let fut = this.block_store.push(block_logs.clone());
                        this.state.set(WatchCanonicalLogsFromState::StoringAdded {
                            pending,
                            next: None,
                            block_logs,
                            fut,
                        });
                        continue;
                    }

                    this.state.set(WatchCanonicalLogsFromState::PollNext);
                }
                WatchCanonicalLogsFromStateProj::StoringAdded { fut, .. } => {
                    match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalLogsFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(())) => {}
                    }
                    let WatchCanonicalLogsFromStateProjOwn::StoringAdded {
                        pending,
                        next,
                        block_logs,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };
                    *this.canonical_tip = Some(block_logs.header().clone());
                    this.state.set(WatchCanonicalLogsFromState::EmitPending { pending, next });
                    return Poll::Ready(Some(Ok(block_log_event(block_logs, false))));
                }
                WatchCanonicalLogsFromStateProj::EmitError { .. } => {
                    let WatchCanonicalLogsFromStateProjOwn::EmitError { err } =
                        this.state.as_mut().project_replace(WatchCanonicalLogsFromState::Done)
                    else {
                        unreachable!()
                    };
                    return Poll::Ready(Some(Err(err)));
                }
            }
        }
    }
}

fn deep_reorg_log_error() -> TransportError {
    TransportErrorKind::custom_str("Deep reorg detected; no canonical log history retained.")
}

fn block_log_event<N: Network>(
    mut block_logs: BlockLogs<N>,
    removed: bool,
) -> CanonicalEvent<BlockLogs<N>> {
    if removed {
        for log in &mut block_logs.logs {
            log.removed = true;
        }
        CanonicalEvent::Removed(block_logs)
    } else {
        // Logs from fresh fetches already have `removed = false`, so no mutation is needed.
        CanonicalEvent::Added(block_logs)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::watch_logs_test_utils::{assert_batch, block, log, MockChain},
        *,
    };
    use crate::{CanonicalStore, Provider};
    use alloy_eips::BlockNumberOrTag;
    use alloy_network::Ethereum;
    use alloy_rpc_types_eth::Filter;
    use futures::StreamExt;
    use std::{collections::HashMap, time::Duration};
    use tokio::time::timeout;

    #[derive(Debug, Default)]
    struct FullHistoryLogStore {
        block_logs: HashMap<u64, BlockLogs<Ethereum>>,
    }

    impl FullHistoryLogStore {
        fn new() -> Self {
            Self::default()
        }
    }

    impl CanonicalStore<BlockLogs<Ethereum>> for FullHistoryLogStore {
        type Error = std::convert::Infallible;
        type PushFuture = std::future::Ready<Result<(), Self::Error>>;
        type GetFuture = std::future::Ready<Result<Option<BlockLogs<Ethereum>>, Self::Error>>;
        type PopFuture = std::future::Ready<Result<Option<BlockLogs<Ethereum>>, Self::Error>>;

        fn push(&mut self, block_logs: BlockLogs<Ethereum>) -> Self::PushFuture {
            self.block_logs.insert(block_logs.header().number(), block_logs);
            std::future::ready(Ok(()))
        }

        fn get(&mut self, block_number: u64) -> Self::GetFuture {
            std::future::ready(Ok(self.block_logs.get(&block_number).cloned()))
        }

        fn pop(&mut self) -> Self::PopFuture {
            let item =
                self.block_logs.keys().max().copied().and_then(|n| self.block_logs.remove(&n));
            std::future::ready(Ok(item))
        }
    }

    async fn next_event<S>(
        stream: &mut WatchCanonicalLogsFromStream<Ethereum, S>,
    ) -> CanonicalEvent<BlockLogs<Ethereum>>
    where
        S: CanonicalStore<BlockLogs<Ethereum>>,
        WatchCanonicalLogsFromStream<Ethereum, S>: Unpin,
    {
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
    async fn custom_block_store_can_recover_log_reorg() {
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
            .block_store(FullHistoryLogStore::new())
            .into_stream();

        assert_added(next_event(&mut stream).await, 1, 1, 1);
        assert_added(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 3, 3, 1);
        assert_added(next_event(&mut stream).await, 4, 4, 1);

        chain.reorg(&[
            (block(2, 22, 1), vec![log(2, 22, 0)]),
            (block(3, 33, 22), vec![log(3, 33, 0)]),
            (block(4, 44, 33), vec![log(4, 44, 0)]),
            (block(5, 55, 44), vec![log(5, 55, 0)]),
        ]);

        assert_removed(next_event(&mut stream).await, 4, 4, 1);
        assert_removed(next_event(&mut stream).await, 3, 3, 1);
        assert_removed(next_event(&mut stream).await, 2, 2, 1);
        assert_added(next_event(&mut stream).await, 2, 22, 1);
        assert_added(next_event(&mut stream).await, 3, 33, 1);
        assert_added(next_event(&mut stream).await, 4, 44, 1);
        assert_added(next_event(&mut stream).await, 5, 55, 1);
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
