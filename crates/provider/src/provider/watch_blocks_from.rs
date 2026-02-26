use super::WatchCanonicalBlocksFrom;
use crate::utils;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::{RpcError, RpcRecv};
use alloy_network::Network;
use alloy_network_primitives::{BlockTransactionsKind, HeaderResponse};
use alloy_primitives::U64;
use alloy_rpc_client::{ClientRef, RpcClientInner, WeakClient};
use alloy_transport::TransportResult;
use futures::{ready, Stream};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::{sleep, Sleep};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::{sleep, Sleep};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

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

    /// Fetches a single block by number.
    pub(super) async fn get_block(&self, block_number: u64) -> TransportResult<N::BlockResponse> {
        get_block::<N>(
            self.client
                .upgrade()
                .ok_or_else(|| RpcError::local_usage_str("provider was dropped"))?,
            block_number,
            self.kind,
            self.poll_interval,
        )
        .await
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
    pub fn into_stream(self) -> WatchBlocksFromStream<N> {
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
/// Each item is a boxed future that, when awaited, fetches one block via
/// `eth_getBlockByNumber`. Callers typically apply
/// [`StreamExt::buffered`](futures::StreamExt::buffered) to resolve
/// multiple block requests concurrently.
pub struct WatchBlocksFromStream<N: Network> {
    inner: WatchBlocksFrom<N>,
    current_block: u64,
    head: u64,
    state: WatchBlocksFromState,
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

enum WatchBlocksFromState {
    /// Upgrade the client and begin fetching head.
    FetchHead,
    /// Polling the in-flight head-block-number future.
    FetchingHead { fut: Pin<Box<dyn Future<Output = TransportResult<u64>> + Send>> },
    /// Yielding block futures for `current_block..=head`.
    Yielding,
    /// Sleeping between poll cycles.
    Sleeping { sleep: Pin<Box<Sleep>> },
    /// Stream terminated.
    Done,
}

type BlockFut<T> = Pin<Box<dyn Future<Output = TransportResult<T>> + Send + 'static>>;

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
                    let block_tag = this.inner.block_tag;
                    let fut: Pin<Box<dyn Future<Output = TransportResult<u64>> + Send>> =
                        Box::pin(async move {
                            fetch_head_block::<N::HeaderResponse>(client.as_ref(), block_tag).await
                        });
                    this.state = WatchBlocksFromState::FetchingHead { fut };
                }
                WatchBlocksFromState::FetchingHead { fut } => match ready!(fut.as_mut().poll(cx)) {
                    Ok(head) => {
                        this.head = head;
                        if this.current_block > head {
                            this.state = WatchBlocksFromState::Sleeping {
                                sleep: Box::pin(sleep(this.inner.poll_interval)),
                            };
                        } else {
                            this.state = WatchBlocksFromState::Yielding;
                        }
                    }
                    Err(err) => {
                        this.state = WatchBlocksFromState::Sleeping {
                            sleep: Box::pin(sleep(this.inner.poll_interval)),
                        };
                        return Poll::Ready(Some(Box::pin(async move { Err(err) })));
                    }
                },
                WatchBlocksFromState::Yielding => {
                    if this.current_block > this.head {
                        this.state = WatchBlocksFromState::Sleeping {
                            sleep: Box::pin(sleep(this.inner.poll_interval)),
                        };
                        continue;
                    }

                    let next_block = this.current_block.saturating_add(1);
                    if next_block <= this.current_block {
                        let err = RpcError::local_usage_str(
                            "watch stream step did not advance block cursor",
                        );
                        this.state = WatchBlocksFromState::Sleeping {
                            sleep: Box::pin(sleep(this.inner.poll_interval)),
                        };
                        return Poll::Ready(Some(Box::pin(async move { Err(err) })));
                    }

                    let Some(client) = this.inner.client.upgrade() else {
                        this.state = WatchBlocksFromState::Done;
                        continue;
                    };

                    let item_fut: BlockFut<N::BlockResponse> = Box::pin(get_block::<N>(
                        client,
                        this.current_block,
                        this.inner.kind,
                        this.inner.poll_interval,
                    ));
                    this.current_block = next_block;
                    return Poll::Ready(Some(item_fut));
                }
                WatchBlocksFromState::Sleeping { sleep } => {
                    ready!(sleep.as_mut().poll(cx));
                    this.state = WatchBlocksFromState::FetchHead;
                }
                WatchBlocksFromState::Done => return Poll::Ready(None),
            }
        }
    }
}

async fn get_block<N: Network>(
    client: impl AsRef<RpcClientInner>,
    block_number: u64,
    kind: BlockTransactionsKind,
    poll_interval: Duration,
) -> TransportResult<N::BlockResponse> {
    loop {
        let block = client
            .as_ref()
            .request("eth_getBlockByNumber", (BlockNumberOrTag::from(block_number), kind.is_full()))
            .await?;
        let block = if kind.is_hashes() { utils::convert_to_hashes(block) } else { block };
        match block {
            Some(block) => return Ok(block),
            None => sleep(poll_interval).await,
        }
    }
}

async fn fetch_head_block<HeaderResp: HeaderResponse + RpcRecv>(
    client: ClientRef<'_>,
    tag: BlockNumberOrTag,
) -> TransportResult<u64> {
    match tag {
        BlockNumberOrTag::Number(number) => Ok(number),
        BlockNumberOrTag::Earliest => Ok(0),
        BlockNumberOrTag::Latest => {
            client.request_noparams::<U64>("eth_blockNumber").await.map(|n| n.to())
        }
        _ => client
            .request::<_, Option<HeaderResp>>("eth_getBlockByNumber", (tag, false))
            .await?
            .map(|header| header.number())
            .ok_or(RpcError::NullResp),
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
