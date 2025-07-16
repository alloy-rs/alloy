use alloy_network::{Ethereum, Network};
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{NoParams, PollerBuilder, WeakClient};
use alloy_transport::{RpcError, TransportErrorKind};
use futures::{Future, Stream, StreamExt};
use lru::LruCache;
use std::{
    marker::PhantomData,
    num::NonZeroUsize,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::{debug, error, trace};

#[cfg(feature = "pubsub")]
use futures::{future::Either, FutureExt};

/// The size of the block cache.
const BLOCK_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(10).unwrap();

/// Maximum number of retries for fetching a block.
const MAX_RETRIES: usize = 3;

/// Default block number for when we don't have a block yet.
const NO_BLOCK_NUMBER: BlockNumber = BlockNumber::MAX;

/// Streams new blocks from the client.
pub(crate) struct NewBlocks<N: Network = Ethereum> {
    client: WeakClient,
    /// The next block to yield.
    /// [`NO_BLOCK_NUMBER`] indicates that it will be updated on the first poll.
    /// Only used by the polling task.
    next_yield: BlockNumber,
    /// LRU cache of known blocks. Only used by the polling task.
    known_blocks: LruCache<BlockNumber, N::BlockResponse>,
    _phantom: PhantomData<N>,
}

/// A stream that converts block numbers into full block responses.
///
/// This stream polls an underlying block number stream and fetches complete
/// block data for each number, maintaining sequential order and handling gaps.
pub(crate) struct BlockStream<N: Network, S> {
    /// The underlying stream of block numbers
    numbers_stream: S,
    /// Weak reference to the RPC client
    client: WeakClient,
    /// The next block number to yield
    next_yield: BlockNumber,
    /// Cache of fetched blocks
    known_blocks: LruCache<BlockNumber, N::BlockResponse>,
    /// Current block fetch future
    current_fetch: Option<
        Pin<
            Box<
                dyn Future<Output = Result<Option<N::BlockResponse>, RpcError<TransportErrorKind>>>
                    + Send,
            >,
        >,
    >,
    /// Current fetch state
    fetch_state: FetchState,
    _phantom: PhantomData<N>,
}

/// State of the current fetch operation
#[derive(Debug, Clone, Copy)]
enum FetchState {
    /// Waiting for a new block number
    WaitingForNumber,
    /// Fetching blocks from `start` to `end` (inclusive)
    Fetching { start: u64, end: u64, current: u64, retries: usize },
}

impl<N: Network> NewBlocks<N> {
    pub(crate) fn new(client: WeakClient) -> Self {
        Self {
            client,
            next_yield: NO_BLOCK_NUMBER,
            known_blocks: LruCache::new(BLOCK_CACHE_SIZE),
            _phantom: PhantomData,
        }
    }

    #[cfg(test)]
    const fn with_next_yield(mut self, next_yield: u64) -> Self {
        self.next_yield = next_yield;
        self
    }

    pub(crate) fn into_stream(self) -> impl Stream<Item = N::BlockResponse> + 'static {
        // Return a stream that lazily subscribes to `newHeads` on the first poll.
        #[cfg(feature = "pubsub")]
        if let Some(client) = self.client.upgrade() {
            if client.pubsub_frontend().is_some() {
                let subscriber = self.into_subscription_stream().map(futures::stream::iter);
                let subscriber = futures::stream::once(subscriber);
                return Either::Left(subscriber.flatten().flatten());
            }
        }

        // Returns a stream that lazily initializes an `eth_blockNumber` polling task on the first
        // poll, mapped with `eth_getBlockByNumber`.
        #[cfg(feature = "pubsub")]
        let right = Either::Right;
        #[cfg(not(feature = "pubsub"))]
        let right = std::convert::identity;
        right(self.into_poll_stream())
    }

    #[cfg(feature = "pubsub")]
    async fn into_subscription_stream(
        self,
    ) -> Option<impl Stream<Item = N::BlockResponse> + 'static> {
        use alloy_consensus::BlockHeader;

        let Some(client) = self.client.upgrade() else {
            debug!("client dropped");
            return None;
        };
        let Some(pubsub) = client.pubsub_frontend() else {
            error!("pubsub_frontend returned None after being Some");
            return None;
        };
        let id = match client.request("eth_subscribe", ("newHeads",)).await {
            Ok(id) => id,
            Err(err) => {
                error!(%err, "failed to subscribe to newHeads");
                return None;
            }
        };
        let sub = match pubsub.get_subscription(id).await {
            Ok(sub) => sub,
            Err(err) => {
                error!(%err, "failed to get subscription");
                return None;
            }
        };
        let stream =
            sub.into_typed::<N::HeaderResponse>().into_stream().map(|header| header.number());
        Some(self.into_block_stream(stream))
    }

    fn into_poll_stream(self) -> impl Stream<Item = N::BlockResponse> + 'static {
        // Spawned lazily on the first `poll`.
        let stream =
            PollerBuilder::<NoParams, U64>::new(self.client.clone(), "eth_blockNumber", [])
                .into_stream()
                .map(|n| n.to());

        self.into_block_stream(stream)
    }

    /// Converts a stream of block numbers into a stream of full block responses.
    ///
    /// This function takes a stream that emits block numbers (either from polling
    /// `eth_blockNumber` or from `newHeads` subscriptions) and transforms it into
    /// a stream of complete block data by fetching each block via `eth_getBlockByNumber`.
    ///
    /// # Arguments
    ///
    /// * `numbers_stream` - A stream that yields block numbers to be fetched
    ///
    /// # Returns
    ///
    /// A `BlockStream` that yields complete block responses (`N::BlockResponse`) for each
    /// block number received from the input stream.
    ///
    /// # Behavior
    ///
    /// - Maintains an internal cache of fetched blocks to handle reorgs and gaps
    /// - Yields blocks sequentially starting from `next_yield`
    /// - Fetches blocks with up to `MAX_RETRIES` attempts for recoverable errors
    /// - Fills gaps by fetching all blocks between the last yielded and the latest
    /// - Stops when the client is dropped or the input stream ends
    fn into_block_stream<S>(self, numbers_stream: S) -> BlockStream<N, S>
    where
        S: Stream<Item = u64> + Unpin + 'static,
    {
        BlockStream {
            numbers_stream,
            client: self.client,
            next_yield: self.next_yield,
            known_blocks: self.known_blocks,
            current_fetch: None,
            fetch_state: FetchState::WaitingForNumber,
            _phantom: PhantomData,
        }
    }
}

impl<N, S> Unpin for BlockStream<N, S>
where
    N: Network,
    S: Unpin,
{
}

impl<N, S> Stream for BlockStream<N, S>
where
    N: Network,
    S: Stream<Item = u64> + Unpin,
{
    type Item = N::BlockResponse;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // First, try to yield any buffered blocks
            if let Some(known_block) = this.known_blocks.pop(&this.next_yield) {
                debug!(number = this.next_yield, "yielding block");
                this.next_yield += 1;
                return Poll::Ready(Some(known_block));
            }

            // If we have a pending fetch, poll it
            if let Some(mut fetch_fut) = this.current_fetch.take() {
                match fetch_fut.as_mut().poll(cx) {
                    Poll::Ready(result) => {
                        if let FetchState::Fetching { start, end, current, retries } =
                            this.fetch_state
                        {
                            match result {
                                Ok(Some(block)) => {
                                    this.known_blocks.put(current, block);

                                    // Check if we've filled the cache or finished the range
                                    if current >= end
                                        || this.known_blocks.len() == BLOCK_CACHE_SIZE.get()
                                    {
                                        this.fetch_state = FetchState::WaitingForNumber;
                                        if this.known_blocks.len() == BLOCK_CACHE_SIZE.get() {
                                            debug!(number = current, "cache full");
                                        }
                                    } else {
                                        // Continue fetching next block
                                        this.fetch_state = FetchState::Fetching {
                                            start,
                                            end,
                                            current: current + 1,
                                            retries: MAX_RETRIES,
                                        };
                                    }
                                }
                                Err(RpcError::Transport(err))
                                    if retries > 0 && err.recoverable() =>
                                {
                                    debug!(number = current, %err, "failed to fetch block, retrying");
                                    this.fetch_state = FetchState::Fetching {
                                        start,
                                        end,
                                        current,
                                        retries: retries - 1,
                                    };
                                }
                                Ok(None) if retries > 0 => {
                                    debug!(
                                        number = current,
                                        "failed to fetch block (doesn't exist), retrying"
                                    );
                                    this.fetch_state = FetchState::Fetching {
                                        start,
                                        end,
                                        current,
                                        retries: retries - 1,
                                    };
                                }
                                Err(err) => {
                                    error!(number = current, %err, "failed to fetch block");
                                    this.fetch_state = FetchState::WaitingForNumber;
                                }
                                Ok(None) => {
                                    error!(
                                        number = current,
                                        "failed to fetch block (doesn't exist)"
                                    );
                                    this.fetch_state = FetchState::WaitingForNumber;
                                }
                            }
                        }
                    }
                    Poll::Pending => {
                        this.current_fetch = Some(fetch_fut);
                        return Poll::Pending;
                    }
                }
            }

            // Handle the current fetch state
            match this.fetch_state {
                FetchState::WaitingForNumber => {
                    // Poll for a new block number
                    match Pin::new(&mut this.numbers_stream).poll_next(cx) {
                        Poll::Ready(Some(block_number)) => {
                            trace!(%block_number, "got block number");

                            if this.next_yield == NO_BLOCK_NUMBER {
                                assert!(block_number < NO_BLOCK_NUMBER, "too many blocks");
                                // this stream can be initialized after the first tx was sent,
                                // to avoid the edge case where the tx is mined immediately, we
                                // should apply an offset to the
                                // initial fetch so that we fetch tip - 1
                                this.next_yield = block_number.saturating_sub(1);
                            } else if block_number < this.next_yield {
                                debug!(
                                    block_number,
                                    next_yield = this.next_yield,
                                    "not advanced yet"
                                );
                                continue;
                            }

                            // Start fetching blocks
                            this.fetch_state = FetchState::Fetching {
                                start: this.next_yield,
                                end: block_number,
                                current: this.next_yield,
                                retries: MAX_RETRIES,
                            };
                        }
                        Poll::Ready(None) => {
                            debug!("polling stream ended");
                            return Poll::Ready(None);
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                FetchState::Fetching { current, .. } => {
                    // Upgrade the provider
                    let Some(client) = this.client.upgrade() else {
                        debug!("client dropped");
                        return Poll::Ready(None);
                    };

                    // Start fetching the current block
                    debug!(number = current, "fetching block");
                    let fut = client.request("eth_getBlockByNumber", (U64::from(current), false));
                    this.current_fetch = Some(Box::pin(fut));
                }
            }
        }
    }
}

#[cfg(all(test, feature = "anvil-api"))] // Tests rely heavily on ability to mine blocks on demand.
mod tests {
    use super::*;
    use crate::{ext::AnvilApi, Provider, ProviderBuilder};
    use alloy_node_bindings::Anvil;
    use std::{future::Future, time::Duration};

    async fn timeout<T: Future>(future: T) -> T::Output {
        try_timeout(future).await.expect("Timeout")
    }

    async fn try_timeout<T: Future>(future: T) -> Option<T::Output> {
        tokio::time::timeout(Duration::from_secs(2), future).await.ok()
    }

    #[tokio::test]
    async fn yield_block_http() {
        yield_block(false).await;
    }
    #[tokio::test]
    #[cfg(feature = "ws")]
    async fn yield_block_ws() {
        yield_block(true).await;
    }
    async fn yield_block(ws: bool) {
        let anvil = Anvil::new().spawn();

        let url = if ws { anvil.ws_endpoint() } else { anvil.endpoint() };
        let provider = ProviderBuilder::new().connect(&url).await.unwrap();

        let new_blocks = NewBlocks::<Ethereum>::new(provider.weak_client()).with_next_yield(1);
        let mut stream = Box::pin(new_blocks.into_stream());
        if ws {
            let _ = try_timeout(stream.next()).await; // Subscribe to newHeads.
        }

        // We will also use provider to manipulate anvil instance via RPC.
        provider.anvil_mine(Some(1), None).await.unwrap();

        let block = timeout(stream.next()).await.expect("Block wasn't fetched");
        assert_eq!(block.header.number, 1);
    }

    #[tokio::test]
    async fn yield_many_blocks_http() {
        yield_many_blocks(false).await;
    }
    #[tokio::test]
    #[cfg(feature = "ws")]
    async fn yield_many_blocks_ws() {
        yield_many_blocks(true).await;
    }
    async fn yield_many_blocks(ws: bool) {
        // Make sure that we can process more blocks than fits in the cache.
        const BLOCKS_TO_MINE: usize = BLOCK_CACHE_SIZE.get() + 1;

        let anvil = Anvil::new().spawn();

        let url = if ws { anvil.ws_endpoint() } else { anvil.endpoint() };
        let provider = ProviderBuilder::new().connect(&url).await.unwrap();

        let new_blocks = NewBlocks::<Ethereum>::new(provider.weak_client()).with_next_yield(1);
        let mut stream = Box::pin(new_blocks.into_stream());
        if ws {
            let _ = try_timeout(stream.next()).await; // Subscribe to newHeads.
        }

        // We will also use provider to manipulate anvil instance via RPC.
        provider.anvil_mine(Some(BLOCKS_TO_MINE as u64), None).await.unwrap();

        let blocks = timeout(stream.take(BLOCKS_TO_MINE).collect::<Vec<_>>()).await;
        assert_eq!(blocks.len(), BLOCKS_TO_MINE);
        let first = blocks[0].header.number;
        assert_eq!(first, 1);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.header.number, first + i as u64);
        }
    }
}
