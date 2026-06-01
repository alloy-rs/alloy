use super::{
    watch_blocks_from::{FetchHeadFut, PollIntervalDelay, DEFAULT_POLL_INTERVAL},
    BlockFut, WatchCanonicalLogsFrom,
};
use crate::transport::TransportErrorKind;
use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::RpcError;
use alloy_network::{BlockResponse as _, Network};
use alloy_network_primitives::{BlockTransactionsKind, HeaderResponse};
use alloy_primitives::B256;
use alloy_rpc_client::{RpcCall, RpcClientInner, WeakClient};
use alloy_rpc_types_eth::{Filter, Log};
use alloy_transport::{TransportError, TransportResult};
use futures::{ready, Stream};
use pin_project::pin_project;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

/// Logs matching a filter for a single block.
///
/// This is the payload resolved by [`BlockLogsFut`] and carried by canonical log events. The
/// `block` field identifies the block whose logs were queried, and `logs` contains all matching
/// logs for that block. `logs` may be empty when the block contained no matching logs.
#[derive(Clone, Debug)]
pub struct BlockLogs<N: Network> {
    /// The block these logs belong to.
    pub block: N::BlockResponse,
    /// Logs matching the configured filter for `block`.
    pub logs: Vec<Log>,
}

impl<N: Network> BlockLogs<N> {
    /// Returns the header for the block these logs belong to.
    pub fn header(&self) -> &N::HeaderResponse {
        self.block.header()
    }
}

/// A builder for streaming block log batches from a historical block.
///
/// `WatchLogsFrom` is the log-specific counterpart to [`super::WatchBlocksFrom`]. It does not
/// perform canonical reconciliation or emit removed events. Instead, it produces a stream of
/// [`BlockLogsFut`] values, one future per block height from `start_block` through the configured
/// `block_tag` head.
///
/// Each future fetches the block and a one-block log range concurrently. Non-empty range results
/// are used when every log matches the fetched block hash; otherwise the future falls back to logs
/// pinned to the fetched block hash. This keeps each resolved batch internally consistent even if
/// the canonical block at that height changes later.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # use alloy_eips::BlockNumberOrTag;
/// # use alloy_provider::{Provider, ProviderBuilder};
/// # use alloy_rpc_types_eth::Filter;
/// # use futures::StreamExt;
///
/// let provider = ProviderBuilder::new().connect_http("http://localhost:8545".parse()?);
/// let filter = Filter::new();
///
/// let mut stream = provider
///     .watch_logs_from(20_000_000, &filter)
///     .block_tag(BlockNumberOrTag::Finalized)
///     .into_stream()
///     .buffered(4);
///
/// while let Some(batch) = stream.next().await {
///     let block_logs = batch?;
///     let _block = block_logs.block;
///     let _logs = block_logs.logs;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchLogsFrom<N: Network> {
    client: WeakClient,
    filter: Filter,
    start_block: u64,
    poll_interval: Duration,
    block_tag: BlockNumberOrTag,
    kind: BlockTransactionsKind,
    _phantom: PhantomData<fn() -> N>,
}

impl<N: Network> WatchLogsFrom<N> {
    pub(crate) const fn new(client: WeakClient, start_block: u64, filter: Filter) -> Self {
        Self {
            client,
            filter,
            start_block,
            poll_interval: DEFAULT_POLL_INTERVAL,
            block_tag: BlockNumberOrTag::Latest,
            kind: BlockTransactionsKind::Hashes,
            _phantom: PhantomData,
        }
    }

    /// Streams block log batches with full transaction bodies in the block response.
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Streams block log batches with transaction hashes only in the block response.
    pub const fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
        self
    }

    /// Sets the poll interval used when the stream is caught up to the configured head tag.
    pub const fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Sets the head block tag used to determine stream progress.
    ///
    /// The stream fetches all block log batches from `start_block` through this head, then polls
    /// again after [`poll_interval`](Self::poll_interval) once caught up.
    pub const fn block_tag(mut self, block_tag: BlockNumberOrTag) -> Self {
        self.block_tag = block_tag;
        self
    }

    /// Converts this builder into a canonical-stream builder that emits
    /// [`CanonicalEvent`](crate::CanonicalEvent) deltas on reorgs.
    pub const fn canonical(self) -> WatchCanonicalLogsFrom<N> {
        WatchCanonicalLogsFrom::new(self)
    }

    /// Creates a future that fetches one block/log batch by block number.
    pub(super) fn get_block_logs(&self, block_number: u64) -> BlockLogsFut<N> {
        self.client
            .upgrade()
            .map(|client| {
                BlockLogsFut::new(
                    client,
                    block_number,
                    self.filter.clone(),
                    self.poll_interval,
                    self.kind,
                )
            })
            .unwrap_or_else(|| BlockLogsFut::err(RpcError::local_usage_str("provider was dropped")))
    }

    /// Stream block/log fetching futures from a historical block.
    ///
    /// The stream polls the configured head, yields futures for `current_block..=head`, then sleeps
    /// for `poll_interval` once caught up. It intentionally mirrors
    /// [`super::WatchBlocksFromStream`] so callers can apply
    /// [`StreamExt::buffered`](futures::StreamExt::buffered) for concurrent RPC work while still
    /// receiving resolved batches in block-number order.
    pub const fn into_stream(self) -> WatchLogsFromStream<N> {
        let current_block = self.start_block;
        WatchLogsFromStream {
            inner: self,
            current_block,
            head: 0,
            state: WatchLogsFromState::FetchHead,
        }
    }
}

/// A stream of block/log-fetching futures.
///
/// Each yielded [`BlockLogsFut`] fetches one block, then fetches matching logs by that block's
/// hash.
#[derive(Debug)]
pub struct WatchLogsFromStream<N: Network> {
    inner: WatchLogsFrom<N>,
    current_block: u64,
    head: u64,
    state: WatchLogsFromState<N>,
}

#[derive(Debug)]
enum WatchLogsFromState<N: Network> {
    /// Upgrade the client and begin fetching the current head.
    FetchHead,
    /// Polling the in-flight head-block-number future.
    FetchingHead { fut: FetchHeadFut<N::HeaderResponse> },
    /// Yielding block/log futures for `current_block..=head`.
    Yielding,
    /// Sleeping between poll cycles.
    Sleeping { delay: PollIntervalDelay },
    /// Stream terminated.
    Done,
}

impl<N: Network> Stream for WatchLogsFromStream<N> {
    type Item = BlockLogsFut<N>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                WatchLogsFromState::FetchHead => {
                    let Some(client) = this.inner.client.upgrade() else {
                        this.state = WatchLogsFromState::Done;
                        continue;
                    };
                    let fut = FetchHeadFut::new(client, this.inner.block_tag);
                    this.state = WatchLogsFromState::FetchingHead { fut };
                }
                WatchLogsFromState::FetchingHead { fut } => match ready!(Pin::new(fut).poll(cx)) {
                    Ok(head) => {
                        this.head = head;
                        if this.current_block > head {
                            this.state = WatchLogsFromState::Sleeping {
                                delay: PollIntervalDelay::new(this.inner.poll_interval),
                            };
                        } else {
                            this.state = WatchLogsFromState::Yielding;
                        }
                    }
                    Err(err) => {
                        this.state = WatchLogsFromState::Sleeping {
                            delay: PollIntervalDelay::new(this.inner.poll_interval),
                        };
                        return Poll::Ready(Some(BlockLogsFut::err(err)));
                    }
                },
                WatchLogsFromState::Yielding => {
                    if this.current_block > this.head {
                        this.state = WatchLogsFromState::Sleeping {
                            delay: PollIntervalDelay::new(this.inner.poll_interval),
                        };
                        continue;
                    }

                    let next_block = this.current_block.saturating_add(1);
                    if next_block <= this.current_block {
                        let err = RpcError::local_usage_str(
                            "watch logs stream step did not advance block cursor",
                        );
                        this.state = WatchLogsFromState::Sleeping {
                            delay: PollIntervalDelay::new(this.inner.poll_interval),
                        };
                        return Poll::Ready(Some(BlockLogsFut::err(err)));
                    }

                    let Some(client) = this.inner.client.upgrade() else {
                        this.state = WatchLogsFromState::Done;
                        continue;
                    };

                    let item_fut = BlockLogsFut::new(
                        client,
                        this.current_block,
                        this.inner.filter.clone(),
                        this.inner.poll_interval,
                        this.inner.kind,
                    );
                    this.current_block = next_block;
                    return Poll::Ready(Some(item_fut));
                }
                WatchLogsFromState::Sleeping { delay } => {
                    ready!(delay.poll(cx));
                    this.state = WatchLogsFromState::FetchHead;
                }
                WatchLogsFromState::Done => return Poll::Ready(None),
            }
        }
    }
}

/// Future that resolves to a block/log batch for one block height.
///
/// `BlockLogsFut` is the item produced by [`WatchLogsFromStream`]. It performs the two pieces of
/// work needed for one height:
///
/// - fetch the block with `eth_getBlockByNumber`;
/// - fetch matching logs for the same numeric height with `eth_getLogs`.
///
/// Those two requests are started concurrently to reduce latency. The numeric-range log result is
/// only used as a fast path when it is non-empty and every returned log carries the fetched block
/// hash. If the range result is empty, has missing/mismatched metadata, or returns an RPC error,
/// the future falls back to a second `eth_getLogs` request pinned to the fetched block hash.
///
/// This means every successful output is internally consistent: `logs` are normalized to the
/// returned block number and block hash. A block-fetch error is returned directly; fallback log
/// errors are returned after the block has been fetched.
#[pin_project]
#[derive(Debug)]
pub struct BlockLogsFut<N: Network> {
    client: Option<Arc<RpcClientInner>>,
    block_number: u64,
    filter: Filter,
    #[pin]
    state: BlockLogsFutState<N>,
}

#[pin_project(project = BlockLogsFutStateProj)]
#[derive(Debug)]
enum BlockLogsFutState<N: Network> {
    /// Polling the candidate block request and optimistic one-block range log request.
    ///
    /// Either RPC may finish first, so completed results are stored in `block` and `range_logs`
    /// until both are ready. The range log result is only accepted if it proves all logs
    /// belong to the fetched block hash; otherwise the future transitions to
    /// [`FetchLogs`](Self::FetchLogs).
    FetchBlockAndLogs {
        block: Option<TransportResult<N::BlockResponse>>,
        range_logs: Option<TransportResult<Vec<Log>>>,
        #[pin]
        block_fut: BlockFut<N::BlockResponse>,
        #[pin]
        logs_call: RpcCall<(Filter,), Vec<Log>>,
    },
    /// Polling fallback logs pinned to the fetched block hash.
    ///
    /// This state is entered when the optimistic range result is empty, ambiguous, mismatched, or
    /// failed. A successful response is normalized against the stored block before the future
    /// resolves.
    FetchLogs {
        block: Option<N::BlockResponse>,
        block_number: u64,
        block_hash: B256,
        #[pin]
        logs_call: RpcCall<(Filter,), Vec<Log>>,
    },
    /// Returning a prebuilt result, used when the parent stream cannot create a normal request.
    Ready { result: Option<TransportResult<BlockLogs<N>>> },
    /// Future has completed and must not be polled again.
    Complete,
}

impl<N: Network> BlockLogsFut<N> {
    fn new(
        client: Arc<RpcClientInner>,
        block_number: u64,
        filter: Filter,
        poll_interval: Duration,
        kind: BlockTransactionsKind,
    ) -> Self {
        let block_fut = Self::block_fut(&client, block_number, poll_interval, kind);
        let logs_call = Self::logs_call(&client, filter.clone().select(block_number));
        Self {
            client: Some(client),
            block_number,
            filter,
            state: BlockLogsFutState::FetchBlockAndLogs {
                block: None,
                range_logs: None,
                block_fut,
                logs_call,
            },
        }
    }

    fn err(err: TransportError) -> Self {
        Self {
            client: None,
            block_number: 0,
            filter: Filter::new(),
            state: BlockLogsFutState::Ready { result: Some(Err(err)) },
        }
    }

    fn block_fut(
        client: &Arc<RpcClientInner>,
        block_number: u64,
        poll_interval: Duration,
        kind: BlockTransactionsKind,
    ) -> BlockFut<N::BlockResponse> {
        BlockFut::new(client.clone(), block_number, kind, poll_interval)
    }

    fn logs_call(client: &Arc<RpcClientInner>, filter: Filter) -> RpcCall<(Filter,), Vec<Log>> {
        client.request("eth_getLogs", (filter,))
    }
}

impl<N: Network> Future for BlockLogsFut<N> {
    type Output = TransportResult<BlockLogs<N>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().project() {
                BlockLogsFutStateProj::FetchBlockAndLogs {
                    block,
                    range_logs,
                    block_fut,
                    logs_call,
                } => {
                    if block.is_none() {
                        if let Poll::Ready(result) = block_fut.poll(cx) {
                            *block = Some(result.and_then(|block| {
                                let block_number = block.header().number();
                                if block_number != *this.block_number {
                                    Err(TransportErrorKind::custom_str(
                                        "eth_getBlockByNumber returned a block with an unexpected number",
                                    ))
                                } else {
                                    Ok(block)
                                }
                            }));
                        }
                    }

                    if range_logs.is_none() {
                        if let Poll::Ready(result) = logs_call.poll(cx) {
                            *range_logs = Some(result);
                        }
                    }

                    if block.as_ref().is_some_and(Result::is_err) {
                        let Some(Err(err)) = block.take() else { unreachable!() };
                        this.state.set(BlockLogsFutState::Complete);
                        return Poll::Ready(Err(err));
                    }

                    if block.is_none() || range_logs.is_none() {
                        return Poll::Pending;
                    }

                    let Ok(block_result) = block.take().expect("checked block is ready") else {
                        unreachable!("block errors are handled above");
                    };
                    let logs_result = range_logs.take().expect("checked logs are ready");

                    let block_number = block_result.header().number();
                    let block_hash = block_result.header().hash();
                    if let Ok(logs) = logs_result {
                        if let Some(logs) =
                            normalize_range_logs_if_matches(logs, block_number, block_hash)
                        {
                            this.state.set(BlockLogsFutState::Complete);
                            return Poll::Ready(Ok(BlockLogs { block: block_result, logs }));
                        }
                    }

                    let Some(client) = this.client.as_ref() else {
                        this.state.set(BlockLogsFutState::Complete);
                        return Poll::Ready(Err(TransportError::local_usage_str(
                            "provider was dropped",
                        )));
                    };
                    let logs_call = BlockLogsFut::<N>::logs_call(
                        client,
                        this.filter.clone().at_block_hash(block_hash),
                    );
                    this.state.set(BlockLogsFutState::FetchLogs {
                        block: Some(block_result),
                        block_number,
                        block_hash,
                        logs_call,
                    });
                }
                BlockLogsFutStateProj::FetchLogs { block, block_number, block_hash, logs_call } => {
                    match ready!(logs_call.poll(cx))
                        .and_then(|logs| normalize_logs(logs, *block_number, *block_hash))
                    {
                        Ok(logs) => {
                            let block = block.take().expect("block is present while fetching logs");
                            this.state.set(BlockLogsFutState::Complete);
                            return Poll::Ready(Ok(BlockLogs { block, logs }));
                        }
                        Err(err) => {
                            this.state.set(BlockLogsFutState::Complete);
                            return Poll::Ready(Err(err));
                        }
                    }
                }
                BlockLogsFutStateProj::Ready { result } => {
                    let result = result.take().expect("polled BlockLogsFut after completion");
                    this.state.set(BlockLogsFutState::Complete);
                    return Poll::Ready(result);
                }
                BlockLogsFutStateProj::Complete => panic!("polled BlockLogsFut after completion"),
            }
        }
    }
}

/// Ensures all logs match the exact block queried and clears any stale removed flag.
fn normalize_logs(
    mut logs: Vec<Log>,
    block_number: u64,
    block_hash: B256,
) -> TransportResult<Vec<Log>> {
    for log in &mut logs {
        if log.block_number.is_some_and(|number| number != block_number) {
            return Err(TransportErrorKind::custom_str(
                "eth_getLogs returned a log with an unexpected block number",
            ));
        }
        if log.block_hash.is_some_and(|hash| hash != block_hash) {
            return Err(TransportErrorKind::custom_str(
                "eth_getLogs returned a log with an unexpected block hash",
            ));
        }
        log.block_number = Some(block_number);
        log.block_hash = Some(block_hash);
        log.removed = false;
    }
    Ok(logs)
}

/// Accepts optimistic range logs only when they prove they belong to the fetched block hash.
fn normalize_range_logs_if_matches(
    logs: Vec<Log>,
    block_number: u64,
    block_hash: B256,
) -> Option<Vec<Log>> {
    if logs.is_empty() {
        return None;
    }

    let all_logs_match = logs.iter().all(|log| {
        log.block_hash == Some(block_hash)
            && !log.block_number.is_some_and(|number| number != block_number)
    });
    if !all_logs_match {
        return None;
    }

    Some(
        normalize_logs(logs, block_number, block_hash)
            .expect("range logs were checked against the target block"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        super::watch_logs_test_utils::{assert_batch, block, log, MockChain},
        *,
    };
    use crate::Provider;
    use alloy_network::Ethereum;
    use alloy_rpc_types_eth::Block;
    use futures::{Stream, StreamExt};
    use tokio::time::timeout;

    async fn next_batch<S>(stream: &mut S) -> BlockLogs<Ethereum>
    where
        S: Stream<Item = TransportResult<BlockLogs<Ethereum>>> + Unpin,
    {
        timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap()
    }

    #[tokio::test]
    async fn streams_log_batches_from_start_block() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)]), (block(2, 2, 1), vec![log(2, 2, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
        assert_batch(&next_batch(&mut stream).await, 2, 2, false, 1);
        assert_eq!(chain.log_request_block_hash_flags(), vec![false, false]);
    }

    #[tokio::test]
    async fn falls_back_to_block_hash_logs_for_empty_range_logs() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![]), (block(2, 2, 1), vec![log(2, 2, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 0);
        assert_batch(&next_batch(&mut stream).await, 2, 2, false, 1);
        assert_eq!(chain.log_request_block_hash_flags(), vec![false, true, false]);
    }

    #[tokio::test]
    async fn falls_back_to_block_hash_logs_for_mismatched_range_logs() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);
        chain.override_next_range_logs(vec![log(1, 11, 0)]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(1))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
        assert_eq!(chain.log_request_block_hash_flags(), vec![false, true]);
    }

    #[tokio::test]
    async fn full_and_hashes_configure_block_fetch_kind() {
        let full_chain = MockChain::new();
        full_chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = full_chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .full()
            .block_tag(BlockNumberOrTag::Number(1))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
        assert_eq!(full_chain.block_request_full_flags(), vec![true]);

        let hashes_chain = MockChain::new();
        hashes_chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = hashes_chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .hashes()
            .block_tag(BlockNumberOrTag::Number(1))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
        assert_eq!(hashes_chain.block_request_full_flags(), vec![false]);
    }

    #[tokio::test]
    async fn provider_retry_layer_retries_log_error_without_skipping_block() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)]), (block(2, 2, 1), vec![log(2, 2, 0)])]);
        chain.fail_next_logs(1);

        let provider = chain.provider_with_retry();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
        assert_batch(&next_batch(&mut stream).await, 2, 2, false, 1);
    }

    #[tokio::test]
    async fn emits_hash_pinned_candidate_when_chain_changes_after_log_fetch() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)]), (block(2, 2, 1), vec![log(2, 2, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);

        chain.reorg_after_next_log_success(vec![
            (block(2, 22, 1), vec![log(2, 22, 0)]),
            (block(3, 33, 22), vec![log(3, 33, 0)]),
        ]);

        assert_batch(&next_batch(&mut stream).await, 2, 2, false, 1);
        assert_batch(&next_batch(&mut stream).await, 3, 33, false, 1);
    }

    #[tokio::test]
    async fn normalizes_log_metadata_and_clears_removed_flag() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![Log { removed: true, ..Default::default() }])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        assert_batch(&next_batch(&mut stream).await, 1, 1, false, 1);
    }

    #[tokio::test]
    async fn rejects_logs_with_conflicting_metadata() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(2, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(format!("{err}").contains("unexpected block number"));
        assert_eq!(chain.log_request_block_hash_flags(), vec![false, true]);
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let chain = MockChain::new();
        let provider = chain.provider();
        let mut stream = provider.watch_logs_from(0, &Filter::new()).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn yielded_future_outlives_provider() {
        let chain = MockChain::new();
        chain.extend(&[(block(1, 1, 0), vec![log(1, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(1, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let fut = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        drop(stream);
        drop(provider);

        let block_logs = timeout(Duration::from_secs(1), fut).await.unwrap().unwrap();
        assert_batch(&block_logs, 1, 1, false, 1);
    }

    #[tokio::test]
    async fn errors_when_cursor_cannot_advance() {
        let chain = MockChain::new();
        let mut block: Block = block(u64::MAX, 1, 0);
        block.header.inner.number = u64::MAX;
        chain.extend(&[(block, vec![log(u64::MAX, 1, 0)])]);

        let provider = chain.provider();
        let mut stream = provider
            .watch_logs_from(u64::MAX, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(u64::MAX))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let err =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap_err();
        assert!(err.is_local_usage_error());
    }
}
