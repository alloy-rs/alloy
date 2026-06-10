use crate::{transport::TransportErrorKind, WatchBlocksFrom, WatchBlocksFromStream};
use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumberOrTag;
use alloy_network::{BlockResponse, Network};
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

/// A builder for streaming canonical block events from a historical block.
///
/// This wraps [`WatchBlocksFrom`] and performs reorg detection: when the chain tip changes
/// incompatibly, the stream yields [`CanonicalEvent::Removed`] for rolled-back blocks
/// followed by [`CanonicalEvent::Added`] for the new canonical chain segment.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchCanonicalBlocksFrom<N, S = InMemoryStore<<N as Network>::BlockResponse>>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
{
    watch_blocks_from: WatchBlocksFrom<N>,
    rpc_concurrency: usize,
    block_store: S,
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
    pub(crate) fn new(watch_blocks_from: WatchBlocksFrom<N>) -> Self {
        Self {
            watch_blocks_from,
            rpc_concurrency: RPC_CONCURRENCY_DEFAULT,
            block_store: InMemoryStore::<N::BlockResponse>::new(MAX_REORG_DEPTH_DEFAULT),
        }
    }
}

impl<N, S> WatchCanonicalBlocksFrom<N, S>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
{
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

    /// Sets the store used to fetch previously emitted canonical blocks during reorg handling.
    pub fn block_store<S2>(self, block_store: S2) -> WatchCanonicalBlocksFrom<N, S2>
    where
        S2: CanonicalStore<N::BlockResponse>,
    {
        WatchCanonicalBlocksFrom {
            watch_blocks_from: self.watch_blocks_from,
            rpc_concurrency: self.rpc_concurrency,
            block_store,
        }
    }

    /// Converts the builder into a stream of canonical block events.
    pub fn into_stream(self) -> WatchCanonicalBlocksFromStream<N, S> {
        let Self { watch_blocks_from, rpc_concurrency, block_store } = self;
        let stream = watch_blocks_from.clone().into_stream().buffered(rpc_concurrency.max(1));

        WatchCanonicalBlocksFromStream {
            watch_blocks_from,
            stream,
            block_store,
            canonical_tip: None,
            state: WatchCanonicalBlocksFromState::PollNext,
        }
    }
}

impl<N: Network> WatchCanonicalBlocksFrom<N, InMemoryStore<N::BlockResponse>> {
    /// Sets the maximum number of canonical blocks retained by the default in-memory store.
    pub fn max_reorg_depth(mut self, max_reorg_depth: usize) -> Self {
        self.block_store = InMemoryStore::<N::BlockResponse>::new(max_reorg_depth);
        self
    }
}

#[pin_project(
    project = WatchCanonicalBlocksFromStateProj,
    project_replace = WatchCanonicalBlocksFromStateProjOwn,
)]
enum WatchCanonicalBlocksFromState<N, S>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
{
    /// Polling the next block from `watch_blocks_from(...).buffered(...)`.
    PollNext,
    /// Reconciling `next` with the canonical buffer by walking parents.
    Reconcile { next: N::BlockResponse, pending: VecDeque<N::BlockResponse> },
    /// Polling an in-flight parent fetch.
    FetchingParent {
        next: N::BlockResponse,
        pending: VecDeque<N::BlockResponse>,
        #[pin]
        fut: super::BlockFut<N::BlockResponse>,
    },
    /// Polling an in-flight fetch for the rolled-back canonical block.
    FetchingRemoved {
        next: N::BlockResponse,
        pending: VecDeque<N::BlockResponse>,
        block_number: u64,
        #[pin]
        fut: S::PopFuture,
    },
    /// Polling an in-flight fetch for the new canonical tip after one rollback.
    FetchingTipAfterRemoved {
        next: N::BlockResponse,
        pending: VecDeque<N::BlockResponse>,
        removed: N::BlockResponse,
        #[pin]
        fut: S::GetFuture,
    },
    /// Emitting `Added` events for `pending`, then `next`.
    EmitPending { pending: VecDeque<N::BlockResponse>, next: Option<N::BlockResponse> },
    /// Polling an in-flight store insert before emitting an `Added` event.
    StoringAdded {
        pending: VecDeque<N::BlockResponse>,
        next: Option<N::BlockResponse>,
        block: N::BlockResponse,
        #[pin]
        fut: S::PushFuture,
    },
    /// Yield one terminal error item and then end the stream.
    EmitError { err: TransportError },
    /// Stream terminated.
    Done,
}

impl<N, S> fmt::Debug for WatchCanonicalBlocksFromState<N, S>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
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
            Self::StoringAdded { pending, next, block, .. } => f
                .debug_struct("StoringAdded")
                .field("pending", pending)
                .field("next", next)
                .field("block", block)
                .finish_non_exhaustive(),
            Self::EmitError { err } => f.debug_struct("EmitError").field("err", err).finish(),
            Self::Done => f.write_str("Done"),
        }
    }
}

/// A stream of canonical block events produced by [`WatchCanonicalBlocksFrom`].
#[derive(Debug)]
#[pin_project]
pub struct WatchCanonicalBlocksFromStream<N, S = InMemoryStore<<N as Network>::BlockResponse>>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
{
    watch_blocks_from: WatchBlocksFrom<N>,
    #[pin]
    stream: Buffered<WatchBlocksFromStream<N>>,
    block_store: S,
    canonical_tip: Option<N::HeaderResponse>,
    #[pin]
    state: WatchCanonicalBlocksFromState<N, S>,
}

impl<N, S> Stream for WatchCanonicalBlocksFromStream<N, S>
where
    N: Network,
    S: CanonicalStore<N::BlockResponse>,
{
    type Item = TransportResult<CanonicalEvent<N::BlockResponse>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().project() {
                WatchCanonicalBlocksFromStateProj::Done => return Poll::Ready(None),
                WatchCanonicalBlocksFromStateProj::PollNext => {
                    match this.stream.as_mut().poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => {
                            this.state.set(WatchCanonicalBlocksFromState::Done);
                        }
                        Poll::Ready(Some(Ok(next))) => {
                            this.state.set(WatchCanonicalBlocksFromState::Reconcile {
                                next,
                                pending: VecDeque::new(),
                            });
                        }
                        Poll::Ready(Some(Err(err))) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError { err });
                        }
                    }
                }
                WatchCanonicalBlocksFromStateProj::Reconcile { .. } => {
                    let WatchCanonicalBlocksFromStateProjOwn::Reconcile { next, pending } =
                        this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };

                    let front = pending.front().unwrap_or(&next);
                    let Some(canonical_tip) = this.canonical_tip.as_ref() else {
                        this.state.set(WatchCanonicalBlocksFromState::EmitPending {
                            pending,
                            next: Some(next),
                        });
                        continue;
                    };

                    let parent_hash = front.header().parent_hash();
                    if parent_hash == canonical_tip.hash() {
                        this.state.set(WatchCanonicalBlocksFromState::EmitPending {
                            pending,
                            next: Some(next),
                        });
                        continue;
                    }

                    // Reorg detected: `front` does not build on canonical tip.
                    // Because WatchBlocksFrom emits strictly sequential heights, we can
                    // remove the tip when heights are adjacent.
                    let height = front.header().number();
                    let canonical_height = canonical_tip.number();
                    if canonical_height + 1 == height {
                        let fut = this.block_store.pop();
                        this.state.set(WatchCanonicalBlocksFromState::FetchingRemoved {
                            next,
                            pending,
                            block_number: canonical_height,
                            fut,
                        });
                        continue;
                    }

                    let Some(parent_height) = height.checked_sub(1) else {
                        this.state.set(WatchCanonicalBlocksFromState::EmitError {
                            err: TransportErrorKind::custom_str(
                                "Cannot backfill parent for genesis block during canonical reconciliation.",
                            ),
                        });
                        continue;
                    };

                    let watch_blocks_from = this.watch_blocks_from.clone();
                    let fut = watch_blocks_from.get_block(parent_height);
                    this.state.set(WatchCanonicalBlocksFromState::FetchingParent {
                        next,
                        pending,
                        fut,
                    });
                }
                WatchCanonicalBlocksFromStateProj::FetchingParent { fut, .. } => {
                    let parent = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError { err });
                            continue;
                        }
                        Poll::Ready(Ok(parent)) => parent,
                    };
                    let WatchCanonicalBlocksFromStateProjOwn::FetchingParent {
                        next,
                        mut pending,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };
                    let front = pending.front().unwrap_or(&next);
                    if parent.header().hash() != front.header().parent_hash() {
                        // Parent no longer matches: a second reorg happened while
                        // reconciling. Abandon this item and continue with next blocks.
                        this.state.set(WatchCanonicalBlocksFromState::PollNext);
                        continue;
                    }
                    pending.push_front(parent);
                    this.state.set(WatchCanonicalBlocksFromState::Reconcile { next, pending });
                }
                WatchCanonicalBlocksFromStateProj::FetchingRemoved { fut, .. } => {
                    let removed = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(None)) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError {
                                err: TransportErrorKind::custom_str(
                                    "Canonical block history is missing an expected block.",
                                ),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(Some(removed))) => removed,
                    };
                    let WatchCanonicalBlocksFromStateProjOwn::FetchingRemoved {
                        next,
                        pending,
                        block_number,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };
                    let Some(parent_number) = block_number.checked_sub(1) else {
                        *this.canonical_tip = None;
                        this.state.set(WatchCanonicalBlocksFromState::EmitError {
                            err: deep_reorg_block_error(),
                        });
                        return Poll::Ready(Some(Ok(CanonicalEvent::Removed(removed))));
                    };
                    let fut = this.block_store.get(parent_number);
                    this.state.set(WatchCanonicalBlocksFromState::FetchingTipAfterRemoved {
                        next,
                        pending,
                        removed,
                        fut,
                    });
                }
                WatchCanonicalBlocksFromStateProj::FetchingTipAfterRemoved { fut, .. } => {
                    let previous_tip = match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(previous_tip)) => previous_tip,
                    };
                    let WatchCanonicalBlocksFromStateProjOwn::FetchingTipAfterRemoved {
                        next,
                        pending,
                        removed,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };
                    *this.canonical_tip = previous_tip.as_ref().map(|block| block.header().clone());
                    if this.canonical_tip.is_some() {
                        this.state.set(WatchCanonicalBlocksFromState::Reconcile { next, pending });
                    } else {
                        this.state.set(WatchCanonicalBlocksFromState::EmitError {
                            err: deep_reorg_block_error(),
                        });
                    }
                    return Poll::Ready(Some(Ok(CanonicalEvent::Removed(removed))));
                }
                WatchCanonicalBlocksFromStateProj::EmitPending { .. } => {
                    let WatchCanonicalBlocksFromStateProjOwn::EmitPending { mut pending, mut next } =
                        this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };

                    if let Some(block) = pending.pop_front() {
                        let fut = this.block_store.push(block.clone());
                        this.state.set(WatchCanonicalBlocksFromState::StoringAdded {
                            pending,
                            next,
                            block,
                            fut,
                        });
                        continue;
                    }

                    if let Some(block) = next.take() {
                        let fut = this.block_store.push(block.clone());
                        this.state.set(WatchCanonicalBlocksFromState::StoringAdded {
                            pending,
                            next: None,
                            block,
                            fut,
                        });
                        continue;
                    }

                    this.state.set(WatchCanonicalBlocksFromState::PollNext);
                }
                WatchCanonicalBlocksFromStateProj::StoringAdded { fut, .. } => {
                    match fut.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.state.set(WatchCanonicalBlocksFromState::EmitError {
                                err: TransportErrorKind::custom(err),
                            });
                            continue;
                        }
                        Poll::Ready(Ok(())) => {}
                    }
                    let WatchCanonicalBlocksFromStateProjOwn::StoringAdded {
                        pending,
                        next,
                        block,
                        ..
                    } = this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };
                    *this.canonical_tip = Some(block.header().clone());
                    this.state.set(WatchCanonicalBlocksFromState::EmitPending { pending, next });
                    return Poll::Ready(Some(Ok(CanonicalEvent::Added(block))));
                }
                WatchCanonicalBlocksFromStateProj::EmitError { .. } => {
                    let WatchCanonicalBlocksFromStateProjOwn::EmitError { err } =
                        this.state.as_mut().project_replace(WatchCanonicalBlocksFromState::Done)
                    else {
                        unreachable!()
                    };
                    return Poll::Ready(Some(Err(err)));
                }
            }
        }
    }
}

/// A store of previously emitted canonical items used to produce removed events on reorgs.
pub trait CanonicalStore<T>: std::fmt::Debug + Send + Sync + 'static {
    /// Error returned by store operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Future returned by [`push`](Self::push).
    #[cfg(not(target_family = "wasm"))]
    type PushFuture: Future<Output = Result<(), Self::Error>> + Send + 'static;

    /// Future returned by [`push`](Self::push).
    #[cfg(target_family = "wasm")]
    type PushFuture: Future<Output = Result<(), Self::Error>> + 'static;

    /// Future returned by [`get`](Self::get).
    #[cfg(not(target_family = "wasm"))]
    type GetFuture: Future<Output = Result<Option<T>, Self::Error>> + Send + 'static;

    /// Future returned by [`get`](Self::get).
    #[cfg(target_family = "wasm")]
    type GetFuture: Future<Output = Result<Option<T>, Self::Error>> + 'static;

    /// Future returned by [`pop`](Self::pop).
    #[cfg(not(target_family = "wasm"))]
    type PopFuture: Future<Output = Result<Option<T>, Self::Error>> + Send + 'static;

    /// Future returned by [`pop`](Self::pop).
    #[cfg(target_family = "wasm")]
    type PopFuture: Future<Output = Result<Option<T>, Self::Error>> + 'static;

    /// Records a newly emitted canonical item.
    ///
    /// Only consecutive items are pushed. If this returns an error, the canonical stream yields
    /// that error and then terminates before emitting the corresponding [`CanonicalEvent::Added`].
    fn push(&mut self, item: T) -> Self::PushFuture;

    /// Fetches a previously emitted canonical item by block number.
    ///
    /// The stream calls this after a rollback to find the new retained tip. If this returns an
    /// error, the stream yields that error and then terminates. Returning `Ok(None)` means the
    /// retained history is missing the requested ancestor; the stream treats that as a deep reorg
    /// and terminates after emitting any already-popped removed item.
    fn get(&mut self, block_number: u64) -> Self::GetFuture;

    /// Removes and returns the most recently pushed canonical item.
    ///
    /// The stream calls this before emitting [`CanonicalEvent::Removed`] to roll back the current
    /// canonical tip. If this returns an error, the stream yields that error and then terminates
    /// before emitting the removed event. Returning `Ok(None)` means the removed block is no longer
    /// available; the stream reports missing canonical history and terminates.
    fn pop(&mut self) -> Self::PopFuture;
}

/// In-memory canonical history store used by default by canonical watchers.
///
/// Stores the most recent `max_reorg_depth` items as a strictly sequential, contiguous chain.
#[derive(Debug)]
pub struct InMemoryStore<T> {
    inner: FixedBuf<T>,
}

impl<T> InMemoryStore<T>
where
    T: BlockResponse,
    T::Header: BlockHeader,
{
    /// Creates an in-memory store retaining up to `max_reorg_depth` canonical items.
    pub fn new(max_reorg_depth: usize) -> Self {
        Self { inner: FixedBuf::new(max_reorg_depth) }
    }
}

/// Error returned by [`InMemoryStore`] operations.
#[derive(Debug, thiserror::Error)]
pub enum InMemoryStoreError {
    /// Pushed item's height leaves a gap relative to the most recent retained item.
    #[error("pushed item #{got} is out of order; expected sequential extension at #{expected}")]
    OutOfOrder {
        /// The block height that was expected next.
        expected: u64,
        /// The block height that was actually provided.
        got: u64,
    },
}

impl<T> CanonicalStore<T> for InMemoryStore<T>
where
    T: BlockResponse + Clone + std::fmt::Debug + Send + Sync + 'static,
    T::Header: BlockHeader,
{
    type Error = InMemoryStoreError;
    type PushFuture = std::future::Ready<Result<(), Self::Error>>;
    type GetFuture = std::future::Ready<Result<Option<T>, Self::Error>>;
    type PopFuture = std::future::Ready<Result<Option<T>, Self::Error>>;

    fn push(&mut self, item: T) -> Self::PushFuture {
        let block_number = item.header().number();
        if let Some(last) = self.inner.last() {
            let expected = last.header().number() + 1;
            if expected != block_number {
                return std::future::ready(Err(InMemoryStoreError::OutOfOrder {
                    expected,
                    got: block_number,
                }));
            }
        }
        self.inner.push(item);
        std::future::ready(Ok(()))
    }

    fn get(&mut self, block_number: u64) -> Self::GetFuture {
        let item = self.inner.last().and_then(|last| {
            let offset = usize::try_from(last.header().number().checked_sub(block_number)?).ok()?;
            let index = self.inner.len().checked_sub(1)?.checked_sub(offset)?;
            self.inner.get(index).and_then(|stored| {
                (stored.header().number() == block_number).then(|| stored.clone())
            })
        });
        std::future::ready(Ok(item))
    }

    fn pop(&mut self) -> Self::PopFuture {
        std::future::ready(Ok(self.inner.pop()))
    }
}

fn deep_reorg_block_error() -> TransportError {
    TransportErrorKind::custom_str("Deep reorg detected; no canonical history retained.")
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

    pub(super) fn get(&self, index: usize) -> Option<&T> {
        self.buf.get(index)
    }

    pub(super) fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockLogs, Provider, ProviderBuilder};
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

    #[derive(Debug, Default)]
    struct FullHistoryBlockStore {
        blocks: HashMap<u64, Block>,
    }

    impl FullHistoryBlockStore {
        fn new() -> Self {
            Self::default()
        }
    }

    impl CanonicalStore<Block> for FullHistoryBlockStore {
        type Error = std::convert::Infallible;
        type PushFuture = std::future::Ready<Result<(), Self::Error>>;
        type GetFuture = std::future::Ready<Result<Option<Block>, Self::Error>>;
        type PopFuture = std::future::Ready<Result<Option<Block>, Self::Error>>;

        fn push(&mut self, block: Block) -> Self::PushFuture {
            let block_number = block.header().number();
            self.blocks.insert(block_number, block);
            std::future::ready(Ok(()))
        }

        fn get(&mut self, block_number: u64) -> Self::GetFuture {
            std::future::ready(Ok(self.blocks.get(&block_number).cloned()))
        }

        fn pop(&mut self) -> Self::PopFuture {
            let item = self.blocks.keys().max().copied().and_then(|n| self.blocks.remove(&n));
            std::future::ready(Ok(item))
        }
    }

    #[tokio::test]
    async fn in_memory_store_supports_block_logs() {
        let mut store = InMemoryStore::<BlockLogs<alloy_network::Ethereum>>::new(2);
        let block_logs = BlockLogs { block: block(1, 1, 0), logs: Vec::new() };

        store.push(block_logs.clone()).await.unwrap();

        let stored = store.get(1).await.unwrap().unwrap();
        assert_eq!(stored.block.header.number, 1);
        assert_eq!(stored.block.header.hash, B256::with_last_byte(1));

        let removed = store.pop().await.unwrap().unwrap();
        assert_eq!(removed.block.header.number, 1);
        assert!(store.get(1).await.unwrap().is_none());
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
    async fn custom_block_store_can_recover_reorg_beyond_default_memory_depth() {
        let chain = MockChain::new();
        chain.extend(&[block(1, 1, 0), block(2, 2, 1), block(3, 3, 2), block(4, 4, 3)]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .canonical()
            .rpc_concurrency(1)
            .block_store(FullHistoryBlockStore::new())
            .into_stream();

        for expected in [1_u64, 2, 3, 4] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => assert_eq!(block.header.number, expected),
                other => panic!("expected Added({expected}), got {other:?}"),
            }
        }

        chain.reorg(&[block(2, 22, 1), block(3, 33, 22), block(4, 44, 33), block(5, 55, 44)]);

        for expected in [(4, 4), (3, 3), (2, 2)] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Removed(block) => {
                    assert_eq!(block.header.number, expected.0);
                    assert_eq!(block.header.hash, B256::with_last_byte(expected.1));
                }
                other => panic!("expected Removed({}), got {other:?}", expected.0),
            }
        }

        for expected in [(2, 22), (3, 33), (4, 44), (5, 55)] {
            let item =
                timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
            match item {
                CanonicalEvent::Added(block) => {
                    assert_eq!(block.header.number, expected.0);
                    assert_eq!(block.header.hash, B256::with_last_byte(expected.1));
                }
                other => panic!("expected Added({}), got {other:?}", expected.0),
            }
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
