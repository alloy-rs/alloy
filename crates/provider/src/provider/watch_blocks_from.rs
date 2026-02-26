use super::WatchCanonicalBlocksFrom;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::{RpcError, RpcRecv};
use alloy_network::{BlockResponse, Network};
use alloy_network_primitives::{BlockTransactionsKind, HeaderResponse};
use alloy_primitives::U64;
use alloy_rpc_client::{RpcCall, RpcClientInner, WeakClient};
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

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::{
    std::Instant,
    tokio::{interval_at, Interval},
};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::{interval_at, Instant, Interval};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
struct PollIntervalDelay {
    timer: Option<Interval>,
}

impl PollIntervalDelay {
    fn new(poll_interval: Duration) -> Self {
        if poll_interval.is_zero() {
            return Self { timer: None };
        }
        Self { timer: Some(interval_at(Instant::now() + poll_interval, poll_interval)) }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(timer) = &mut self.timer {
            ready!(timer.poll_tick(cx));
        }
        Poll::Ready(())
    }
}

/// Future returned by [`WatchBlocksFromStream`] items.
#[pin_project]
#[derive(Debug)]
pub struct BlockFut<T>
where
    T: BlockResponse + RpcRecv,
{
    client: Option<Arc<RpcClientInner>>,
    block_number: u64,
    kind: BlockTransactionsKind,
    poll_interval: Duration,
    #[pin]
    state: BlockFutState<T>,
}

#[pin_project(project = BlockFutStateProj)]
#[derive(Debug)]
enum BlockFutState<T>
where
    T: BlockResponse + RpcRecv,
{
    Request {
        #[pin]
        call: RpcCall<(BlockNumberOrTag, bool), Option<T>>,
    },
    Sleeping {
        delay: PollIntervalDelay,
    },
    Ready {
        result: Option<TransportResult<T>>,
    },
    Complete,
}

impl<T> BlockFut<T>
where
    T: BlockResponse + RpcRecv,
{
    pub(super) fn new(
        client: Arc<RpcClientInner>,
        block_number: u64,
        kind: BlockTransactionsKind,
        poll_interval: Duration,
    ) -> Self {
        let call = Self::block_request_call(&client, block_number, kind);
        Self {
            client: Some(client),
            block_number,
            kind,
            poll_interval,
            state: BlockFutState::Request { call },
        }
    }

    pub(super) const fn err(err: TransportError) -> Self {
        Self {
            client: None,
            block_number: 0,
            kind: BlockTransactionsKind::Hashes,
            poll_interval: Duration::from_secs(0),
            state: BlockFutState::Ready { result: Some(Err(err)) },
        }
    }

    fn block_request_call(
        client: &Arc<RpcClientInner>,
        block_number: u64,
        kind: BlockTransactionsKind,
    ) -> RpcCall<(BlockNumberOrTag, bool), Option<T>> {
        client
            .request("eth_getBlockByNumber", (BlockNumberOrTag::from(block_number), kind.is_full()))
    }
}

impl<T> Future for BlockFut<T>
where
    T: BlockResponse + RpcRecv,
{
    type Output = TransportResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().project() {
                BlockFutStateProj::Request { call } => match ready!(call.poll(cx)) {
                    Ok(Some(mut block)) => {
                        if this.kind.is_hashes() && block.transactions().is_empty() {
                            block.transactions_mut().convert_to_hashes();
                        }
                        this.state.set(BlockFutState::Complete);
                        return Poll::Ready(Ok(block));
                    }
                    Ok(None) => {
                        this.state.set(BlockFutState::Sleeping {
                            delay: PollIntervalDelay::new(*this.poll_interval),
                        });
                    }
                    Err(err) => {
                        this.state.set(BlockFutState::Complete);
                        return Poll::Ready(Err(err));
                    }
                },
                BlockFutStateProj::Sleeping { delay } => {
                    ready!(delay.poll(cx));
                    let Some(client) = this.client.as_ref() else {
                        this.state.set(BlockFutState::Complete);
                        return Poll::Ready(Err(TransportError::local_usage_str(
                            "provider was dropped",
                        )));
                    };
                    this.state.set(BlockFutState::Request {
                        call: Self::block_request_call(client, *this.block_number, *this.kind),
                    });
                }
                BlockFutStateProj::Ready { result } => {
                    let result = result.take().expect("polled BlockFut after completion");
                    this.state.set(BlockFutState::Complete);
                    return Poll::Ready(result);
                }
                BlockFutStateProj::Complete => panic!("polled BlockFut after completion"),
            }
        }
    }
}

#[pin_project]
#[derive(Debug)]
struct FetchHeadFut<HeaderResp>
where
    HeaderResp: HeaderResponse + RpcRecv,
{
    #[pin]
    state: FetchHeadFutState<HeaderResp>,
}

#[pin_project(project = FetchHeadFutStateProj)]
#[derive(Debug)]
enum FetchHeadFutState<HeaderResp>
where
    HeaderResp: HeaderResponse + RpcRecv,
{
    Latest {
        #[pin]
        call: RpcCall<[(); 0], U64>,
    },
    Tagged {
        #[pin]
        call: RpcCall<(BlockNumberOrTag, bool), Option<HeaderResp>>,
    },
    Ready {
        result: Option<TransportResult<u64>>,
    },
    Complete,
}

impl<HeaderResp> FetchHeadFut<HeaderResp>
where
    HeaderResp: HeaderResponse + RpcRecv,
{
    fn new(client: Arc<RpcClientInner>, tag: BlockNumberOrTag) -> Self {
        let state = match tag {
            BlockNumberOrTag::Number(number) => {
                FetchHeadFutState::Ready { result: Some(Ok(number)) }
            }
            BlockNumberOrTag::Earliest => FetchHeadFutState::Ready { result: Some(Ok(0)) },
            BlockNumberOrTag::Latest => {
                FetchHeadFutState::Latest { call: client.request_noparams("eth_blockNumber") }
            }
            _ => FetchHeadFutState::Tagged {
                call: client.request("eth_getBlockByNumber", (tag, false)),
            },
        };
        Self { state }
    }
}

impl<HeaderResp> Future for FetchHeadFut<HeaderResp>
where
    HeaderResp: HeaderResponse + RpcRecv,
{
    type Output = TransportResult<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        match this.state.as_mut().project() {
            FetchHeadFutStateProj::Latest { call } => {
                let result = ready!(call.poll(cx)).map(|n| n.to());
                this.state.set(FetchHeadFutState::Complete);
                Poll::Ready(result)
            }
            FetchHeadFutStateProj::Tagged { call } => {
                let result = match ready!(call.poll(cx)) {
                    Ok(resp) => resp.map(|header| header.number()).ok_or(RpcError::NullResp),
                    Err(err) => Err(err),
                };
                this.state.set(FetchHeadFutState::Complete);
                Poll::Ready(result)
            }
            FetchHeadFutStateProj::Ready { result } => {
                let result = result.take().expect("polled FetchHeadFut after completion");
                this.state.set(FetchHeadFutState::Complete);
                Poll::Ready(result)
            }
            FetchHeadFutStateProj::Complete => panic!("polled FetchHeadFut after completion"),
        }
    }
}

/// A builder for streaming blocks from a historical block and continuing indefinitely.
#[derive(Debug, Clone)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchBlocksFrom<N: Network> {
    client: WeakClient,
    start_block: u64,
    poll_interval: Duration,
    block_tag: BlockNumberOrTag,
    kind: BlockTransactionsKind,
    _phantom: PhantomData<fn() -> N>,
}

impl<N: Network> WatchBlocksFrom<N> {
    /// Creates a new [`WatchBlocksFrom`] builder.
    pub(crate) const fn new(client: WeakClient, start_block: u64) -> Self {
        Self {
            client,
            start_block,
            poll_interval: DEFAULT_POLL_INTERVAL,
            block_tag: BlockNumberOrTag::Finalized,
            kind: BlockTransactionsKind::Hashes,
            _phantom: PhantomData,
        }
    }

    /// Streams blocks with full transaction bodies.
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Streams blocks with transaction hashes only.
    pub const fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
        self
    }

    /// Sets the poll interval used when the stream is caught up.
    pub const fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Sets the head block tag used to determine stream progress.
    pub const fn block_tag(mut self, block_tag: BlockNumberOrTag) -> Self {
        self.block_tag = block_tag;
        self
    }

    /// Converts this builder into a canonical-stream builder that emits
    /// [`crate::CanonicalEvent`] deltas on reorgs.
    pub const fn canonical(self) -> WatchCanonicalBlocksFrom<N> {
        WatchCanonicalBlocksFrom::new(self)
    }

    /// Creates a future that fetches a single block by number.
    pub(super) fn get_block(&self, block_number: u64) -> BlockFut<N::BlockResponse> {
        self.client
            .upgrade()
            .map(|client| BlockFut::new(client, block_number, self.kind, self.poll_interval))
            .unwrap_or_else(|| BlockFut::err(RpcError::local_usage_str("provider was dropped")))
    }

    /// Stream blocks from a historical block using sequential `eth_getBlockByNumber` calls.
    ///
    /// This stream continues polling after catching up and continues yielding new blocks
    /// indefinitely.
    ///
    /// This stream _does not_ handle reorgs. Instead, each item yielded from the stream
    /// is strictly ordered in terms of block number, regardless of the blocks parent.
    ///
    /// For example (height, hash, parent):
    ///
    /// You should expect blocks in order by number with no gaps and with disjoint parents:
    /// [(1, 1A, 0A),(2, 2A, 1A),(3,3B,2B)]
    ///
    /// And you should not expect receiving two blocks with the same number:
    /// [(1, 1A, 0A),(2, 2A, 1A),(2,2B,1A)]
    ///
    /// Each yielded future contains one block request.
    ///
    /// If a block request returns `NullResp`, the yielded future retries the same block until it
    /// succeeds.
    ///
    /// Other errors are surfaced to the caller. Configure retries on the underlying client
    /// transport (for example with `RetryBackoffLayer`) for transport-level retry behavior.
    ///
    /// This can be buffered by the caller, for example with
    /// [`StreamExt::buffered`](futures::StreamExt::buffered).
    pub const fn into_stream(self) -> WatchBlocksFromStream<N> {
        let current_block = self.start_block;
        WatchBlocksFromStream {
            inner: self,
            current_block,
            head: 0,
            state: WatchBlocksFromState::FetchHead,
        }
    }
}

/// A stream of block-fetching futures produced by [`WatchBlocksFrom`].
///
/// Each item is a [`BlockFut`] that, when awaited, fetches one block via
/// `eth_getBlockByNumber`. Callers typically apply
/// [`StreamExt::buffered`](futures::StreamExt::buffered) to resolve
/// multiple block requests concurrently.
pub struct WatchBlocksFromStream<N: Network> {
    inner: WatchBlocksFrom<N>,
    current_block: u64,
    head: u64,
    state: WatchBlocksFromState<N>,
}

impl<N: Network> std::fmt::Debug for WatchBlocksFromStream<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchBlocksFromStream")
            .field("current_block", &self.current_block)
            .field("poll_interval", &self.inner.poll_interval)
            .field("block_tag", &self.inner.block_tag)
            .field("kind", &self.inner.kind)
            .finish_non_exhaustive()
    }
}

enum WatchBlocksFromState<N: Network> {
    /// Upgrade the client and begin fetching head.
    FetchHead,
    /// Polling the in-flight head-block-number future.
    FetchingHead { fut: FetchHeadFut<N::HeaderResponse> },
    /// Yielding block futures for `current_block..=head`.
    Yielding,
    /// Sleeping between poll cycles.
    Sleeping { delay: PollIntervalDelay },
    /// Stream terminated.
    Done,
}

impl<N: Network> Stream for WatchBlocksFromStream<N> {
    type Item = BlockFut<N::BlockResponse>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                WatchBlocksFromState::FetchHead => {
                    let Some(client) = this.inner.client.upgrade() else {
                        this.state = WatchBlocksFromState::Done;
                        continue;
                    };
                    let fut = FetchHeadFut::new(client, this.inner.block_tag);
                    this.state = WatchBlocksFromState::FetchingHead { fut };
                }
                WatchBlocksFromState::FetchingHead { fut } => {
                    match ready!(Pin::new(fut).poll(cx)) {
                        Ok(head) => {
                            this.head = head;
                            if this.current_block > head {
                                this.state = WatchBlocksFromState::Sleeping {
                                    delay: PollIntervalDelay::new(this.inner.poll_interval),
                                };
                            } else {
                                this.state = WatchBlocksFromState::Yielding;
                            }
                        }
                        Err(err) => {
                            this.state = WatchBlocksFromState::Sleeping {
                                delay: PollIntervalDelay::new(this.inner.poll_interval),
                            };
                            return Poll::Ready(Some(BlockFut::err(err)));
                        }
                    }
                }
                WatchBlocksFromState::Yielding => {
                    if this.current_block > this.head {
                        this.state = WatchBlocksFromState::Sleeping {
                            delay: PollIntervalDelay::new(this.inner.poll_interval),
                        };
                        continue;
                    }

                    let next_block = this.current_block.saturating_add(1);
                    if next_block <= this.current_block {
                        let err = RpcError::local_usage_str(
                            "watch stream step did not advance block cursor",
                        );
                        this.state = WatchBlocksFromState::Sleeping {
                            delay: PollIntervalDelay::new(this.inner.poll_interval),
                        };
                        return Poll::Ready(Some(BlockFut::err(err)));
                    }

                    let Some(client) = this.inner.client.upgrade() else {
                        this.state = WatchBlocksFromState::Done;
                        continue;
                    };

                    let item_fut: BlockFut<N::BlockResponse> = BlockFut::new(
                        client,
                        this.current_block,
                        this.inner.kind,
                        this.inner.poll_interval,
                    );
                    this.current_block = next_block;
                    return Poll::Ready(Some(item_fut));
                }
                WatchBlocksFromState::Sleeping { delay } => {
                    ready!(delay.poll(cx));
                    this.state = WatchBlocksFromState::FetchHead;
                }
                WatchBlocksFromState::Done => return Poll::Ready(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::Block;
    use alloy_transport::{
        layers::{RetryBackoffLayer, RetryPolicy},
        mock::MockTransport,
    };
    use futures::StreamExt;
    use tokio::time::timeout;

    fn block(number: u64) -> Block {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block
    }

    #[tokio::test]
    async fn streams_blocks_from_start_block() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&3_u64);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&Some(block(2)));
        asserter.push_success(&Some(block(3)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);

        let third = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(third.header.number, 3);
    }

    #[tokio::test]
    async fn advances_to_next_block_after_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&1_u64);
        asserter.push_failure_msg("boom");
        asserter.push_success(&2_u64);
        asserter.push_success(&Some(block(2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);
    }

    #[tokio::test]
    async fn retries_same_block_after_null_response() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let no_block: Option<Block> = None;
        asserter.push_success(&1_u64);
        asserter.push_success(&no_block);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&2_u64);
        asserter.push_success(&Some(block(2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);
    }

    #[tokio::test]
    async fn recovers_after_head_fetch_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_failure_msg("head boom");
        asserter.push_success(&1_u64);
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 1);
    }

    #[tokio::test]
    async fn uses_provider_retry_layer() {
        #[derive(Clone, Debug)]
        struct AlwaysRetryPolicy;

        impl RetryPolicy for AlwaysRetryPolicy {
            fn should_retry(&self, _error: &alloy_transport::TransportError) -> bool {
                true
            }

            fn backoff_hint(&self, _error: &alloy_transport::TransportError) -> Option<Duration> {
                None
            }
        }

        let asserter = alloy_transport::mock::Asserter::new();
        let retry_layer = RetryBackoffLayer::new_with_policy(3, 0, 10_000, AlwaysRetryPolicy);
        let client = RpcClient::builder()
            .layer(retry_layer)
            .transport(MockTransport::new(asserter.clone()), true);
        let provider = ProviderBuilder::new().connect_client(client);

        asserter.push_failure_msg("temporary head error");
        asserter.push_success(&1_u64);
        asserter.push_failure_msg("temporary block error");
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);
    }

    #[tokio::test]
    async fn waits_until_head_reaches_start_block() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&0_u64);
        asserter.push_success(&1_u64);
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);
    }

    #[tokio::test]
    async fn fixed_block_tag_number_does_not_fetch_head() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&Some(block(5)));

        let mut stream = provider
            .watch_blocks_from(5)
            .block_tag(BlockNumberOrTag::Number(5))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 5);
    }

    #[tokio::test]
    async fn earliest_block_tag_starts_at_zero() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&Some(block(0)));

        let mut stream = provider
            .watch_blocks_from(0)
            .block_tag(BlockNumberOrTag::Earliest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 0);
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let provider =
            ProviderBuilder::new().connect_mocked_client(alloy_transport::mock::Asserter::new());
        let mut stream = provider.watch_blocks_from(0).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn yielded_future_outlives_provider() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&1_u64);
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let fut = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        drop(stream);
        drop(provider);

        let block = timeout(Duration::from_secs(1), fut).await.unwrap().unwrap();
        assert_eq!(block.header.number, 1);
    }

    #[tokio::test]
    async fn multiple_yielded_futures_outlive_provider() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&2_u64);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&Some(block(2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let fut1 = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        let fut2 = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        drop(provider);

        let first = timeout(Duration::from_secs(1), fut1).await.unwrap().unwrap();
        let second = timeout(Duration::from_secs(1), fut2).await.unwrap().unwrap();
        assert_eq!(first.header.number, 1);
        assert_eq!(second.header.number, 2);
    }

    #[tokio::test]
    async fn errors_when_cursor_cannot_advance() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);

        let mut stream = provider
            .watch_blocks_from(u64::MAX)
            .block_tag(BlockNumberOrTag::Number(u64::MAX))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        let err = first.unwrap_err();
        assert!(err.is_local_usage_error());
    }

    #[tokio::test]
    async fn future_stream_can_be_buffered() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&2_u64);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&Some(block(2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(2);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);
    }

    #[tokio::test]
    async fn buffered_stream_does_not_skip_after_null_response() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let no_block: Option<Block> = None;
        asserter.push_success(&2_u64);
        asserter.push_success(&no_block);
        asserter.push_success(&Some(block(2)));
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(2);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);
    }
}
