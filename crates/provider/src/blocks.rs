use alloy_network::{Ethereum, Network};
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{NoParams, PollerBuilder, WeakClient};
use alloy_transport::RpcError;
use futures::{ready, Future, FutureExt, Stream, StreamExt};
use lru::LruCache;
use std::{
    marker::PhantomData, 
    num::NonZeroUsize,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "pubsub")]
use futures::future::Either;

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

    pub(crate) fn into_stream(self) -> NewBlocksStream<N> {
        // Return a stream that lazily subscribes to `newHeads` on the first poll.
        #[cfg(feature = "pubsub")]
        if let Some(client) = self.client.upgrade() {
            if client.pubsub_frontend().is_some() {
                return NewBlocksStream::Subscription(Box::pin(async move {
                    match self.into_subscription_stream().await {
                        Some(stream) => Some(Box::pin(stream) as Pin<Box<dyn Stream<Item = N::BlockResponse> + Send>>),
                        None => None,
                    }
                }));
            }
        }

        // Returns a stream that lazily initializes an `eth_blockNumber` polling task on the first
        // poll, mapped with `eth_getBlockByNumber`.
        NewBlocksStream::Polling(self.into_poll_stream())
    }

    #[cfg(feature = "pubsub")]
    async fn into_subscription_stream(
        self,
    ) -> Option<Pin<Box<dyn Stream<Item = N::BlockResponse> + Send>>> {
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
        let header_stream = sub.into_typed::<N::HeaderResponse>().into_stream().map(|header| header.number());
        let block_stream = self.into_block_stream(Box::new(header_stream) as Box<dyn Stream<Item = u64> + Send + Unpin>);
        Some(Box::pin(block_stream) as Pin<Box<dyn Stream<Item = N::BlockResponse> + Send>>)
    }

    fn into_poll_stream(self) -> BlockStream<N, Box<dyn Stream<Item = u64> + Send + Unpin>> {
        // Spawned lazily on the first `poll`.
        let stream =
            PollerBuilder::<NoParams, U64>::new(self.client.clone(), "eth_blockNumber", [])
                .into_stream()
                .map(|n| n.to());

        self.into_block_stream(Box::new(stream) as Box<dyn Stream<Item = u64> + Send + Unpin>)
    }

    fn into_block_stream<S>(self, numbers_stream: S) -> BlockStream<N, S>
    where
        S: Stream<Item = u64> + Unpin + 'static,
    {
        BlockStream::new(self, numbers_stream)
    }
}

/// State for fetching blocks.
enum BlockFetchState<N: Network> {
    /// Yielding buffered blocks.
    YieldingBuffered,
    /// Waiting for next block number.
    WaitingForNumber,
    /// Fetching blocks from the client.
    Fetching {
        client: std::sync::Arc<alloy_rpc_client::RpcClientInner>,
        target_block: BlockNumber,
        current_number: BlockNumber,
        retries: usize,
        fut: Option<Pin<Box<dyn Future<Output = Result<Option<N::BlockResponse>, RpcError<alloy_transport::TransportErrorKind>>> + Send>>>,
    },
}

/// A stream that yields blocks by fetching them from the client.
pub(crate) struct BlockStream<N: Network, S> {
    /// The underlying block numbers stream.
    numbers_stream: S,
    /// The client to fetch blocks with.
    client: WeakClient,
    /// The next block to yield.
    next_yield: BlockNumber,
    /// LRU cache of known blocks.
    known_blocks: LruCache<BlockNumber, N::BlockResponse>,
    /// Current state of the stream.
    state: BlockFetchState<N>,
    _phantom: PhantomData<N>,
}

impl<N: Network, S> BlockStream<N, S> {
    fn new(new_blocks: NewBlocks<N>, numbers_stream: S) -> Self {
        Self {
            numbers_stream,
            client: new_blocks.client,
            next_yield: new_blocks.next_yield,
            known_blocks: new_blocks.known_blocks,
            state: BlockFetchState::YieldingBuffered,
            _phantom: PhantomData,
        }
    }
}

impl<N: Network, S> Unpin for BlockStream<N, S> {}

impl<N: Network, S> Stream for BlockStream<N, S>
where
    S: Stream<Item = u64> + Unpin,
{
    type Item = N::BlockResponse;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                BlockFetchState::YieldingBuffered => {
                    // Clear any buffered blocks.
                    if let Some(known_block) = this.known_blocks.pop(&this.next_yield) {
                        debug!(number=this.next_yield, "yielding block");
                        this.next_yield += 1;
                        return Poll::Ready(Some(known_block));
                    }
                    // No more buffered blocks, wait for next number.
                    this.state = BlockFetchState::WaitingForNumber;
                }
                BlockFetchState::WaitingForNumber => {
                    // Get the tip.
                    match ready!(this.numbers_stream.poll_next_unpin(cx)) {
                        Some(block_number) => {
                            trace!(%block_number, "got block number");
                            if this.next_yield == NO_BLOCK_NUMBER {
                                assert!(block_number < NO_BLOCK_NUMBER, "too many blocks");
                                this.next_yield = block_number;
                            } else if block_number < this.next_yield {
                                debug!(block_number, this.next_yield, "not advanced yet");
                                continue;
                            }

                            // Upgrade the client.
                            let Some(client) = this.client.upgrade() else {
                                debug!("client dropped");
                                return Poll::Ready(None);
                            };

                            // Start fetching blocks.
                            this.state = BlockFetchState::Fetching {
                                client,
                                target_block: block_number,
                                current_number: this.next_yield,
                                retries: MAX_RETRIES,
                                fut: None,
                            };
                        }
                        None => {
                            debug!("polling stream ended");
                            return Poll::Ready(None);
                        }
                    }
                }
                BlockFetchState::Fetching { client, target_block, current_number, retries, fut } => {
                    if let Some(future) = fut {
                        // Poll the ongoing request.
                        match ready!(future.poll_unpin(cx)) {
                            Ok(Some(block)) => {
                                let number = *current_number;
                                this.known_blocks.put(number, block);
                                *current_number += 1;
                                *fut = None;
                                
                                if this.known_blocks.len() == BLOCK_CACHE_SIZE.get() {
                                    // Cache is full, should be consumed before filling more blocks.
                                    debug!(number, "cache full");
                                    this.state = BlockFetchState::YieldingBuffered;
                                    continue;
                                }
                            }
                            Err(RpcError::Transport(err)) if *retries > 0 && err.recoverable() => {
                                debug!(number=*current_number, %err, "failed to fetch block, retrying");
                                *retries -= 1;
                                *fut = None;
                            }
                            Ok(None) if *retries > 0 => {
                                debug!(number=*current_number, "failed to fetch block (doesn't exist), retrying");
                                *retries -= 1;
                                *fut = None;
                            }
                            Err(err) => {
                                error!(number=*current_number, %err, "failed to fetch block");
                                this.state = BlockFetchState::YieldingBuffered;
                                continue;
                            }
                            Ok(None) => {
                                error!(number=*current_number, "failed to fetch block (doesn't exist)");
                                this.state = BlockFetchState::YieldingBuffered;
                                continue;
                            }
                        }
                    }

                    // Check if we're done fetching.
                    if *current_number > *target_block {
                        this.state = BlockFetchState::YieldingBuffered;
                        continue;
                    }

                    // Start a new request.
                    debug!(number=*current_number, "fetching block");
                    let client_ref = client.clone();
                    let number = *current_number;
                    let future = Box::pin(async move {
                        client_ref.request("eth_getBlockByNumber", (U64::from(number), false)).await
                    });
                    *fut = Some(future);
                }
            }
        }
    }
}

/// A stream that yields new blocks.
pub(crate) enum NewBlocksStream<N: Network> {
    /// Polling-based stream.
    Polling(BlockStream<N, Box<dyn Stream<Item = u64> + Send + Unpin>>),
    /// Subscription-based stream (WebSocket) - initial state.
    #[cfg(feature = "pubsub")]
    Subscription(Pin<Box<dyn Future<Output = Option<Pin<Box<dyn Stream<Item = N::BlockResponse> + Send>>>> + Send>>),
    /// Subscription-based stream (WebSocket) - ready state.
    #[cfg(feature = "pubsub")]
    SubscriptionReady(Pin<Box<dyn Stream<Item = N::BlockResponse> + Send>>),
}

impl<N: Network> Stream for NewBlocksStream<N> {
    type Item = N::BlockResponse;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            NewBlocksStream::Polling(stream) => stream.poll_next_unpin(cx),
            #[cfg(feature = "pubsub")]
            NewBlocksStream::Subscription(fut) => {
                // First poll the future to get the stream
                match ready!(fut.poll_unpin(cx)) {
                    Some(mut stream) => {
                        // Replace self with the actual stream for future polls
                        match stream.poll_next_unpin(cx) {
                            Poll::Ready(item) => {
                                // Continue using the stream
                                *self = NewBlocksStream::SubscriptionReady(stream);
                                Poll::Ready(item)
                            }
                            Poll::Pending => {
                                *self = NewBlocksStream::SubscriptionReady(stream);
                                Poll::Pending
                            }
                        }
                    }
                    None => Poll::Ready(None),
                }
            }
            #[cfg(feature = "pubsub")]
            NewBlocksStream::SubscriptionReady(stream) => stream.poll_next_unpin(cx),
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
