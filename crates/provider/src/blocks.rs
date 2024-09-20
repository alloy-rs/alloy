use alloy_network::{Ethereum, Network};
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{NoParams, PollerBuilder, WeakClient};
use alloy_transport::{RpcError, Transport, TransportResult};
use async_stream::stream;
use futures::{future::Either, Stream, StreamExt};
use lru::LruCache;
use std::{marker::PhantomData, num::NonZeroUsize};

/// The size of the block cache.
const BLOCK_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(10) };

/// Maximum number of retries for fetching a block.
const MAX_RETRIES: usize = 3;

/// Default block number for when we don't have a block yet.
const NO_BLOCK_NUMBER: BlockNumber = BlockNumber::MAX;

/// Streams new blocks from the client.
pub(crate) struct NewBlocks<T, N: Network = Ethereum> {
    client: WeakClient<T>,
    next_yield: BlockNumber,
    known_blocks: LruCache<BlockNumber, N::BlockResponse>,
    _phantom: PhantomData<N>,
}

impl<T: Transport + Clone, N: Network> NewBlocks<T, N> {
    pub(crate) fn new(client: WeakClient<T>) -> Self {
        Self {
            client: client.clone(),
            next_yield: NO_BLOCK_NUMBER,
            known_blocks: LruCache::new(BLOCK_CACHE_SIZE),
            _phantom: PhantomData,
        }
    }

    pub(crate) async fn into_stream(
        self,
    ) -> TransportResult<impl Stream<Item = N::BlockResponse> + 'static> {
        #[cfg(feature = "pubsub")]
        if let Some(client) = self.client.upgrade() {
            if let Some(pubsub) = client.pubsub_frontend() {
                let id = client.request("eth_subscribe", ("newHeads",)).await?;
                let sub = pubsub.get_subscription(id).await?;
                return Ok(Either::Left(sub.into_typed::<N::BlockResponse>().into_stream()));
            }
        }

        #[cfg(feature = "pubsub")]
        let right = Either::Right;
        #[cfg(not(feature = "pubsub"))]
        let right = std::convert::identity;
        Ok(right(self.into_poll_stream()))
    }

    fn into_poll_stream(mut self) -> impl Stream<Item = N::BlockResponse> + 'static {
        let poll_task_builder: PollerBuilder<T, NoParams, U64> =
            PollerBuilder::new(self.client.clone(), "eth_blockNumber", []);
        let mut poll_task = poll_task_builder.spawn().into_stream_raw();
        stream! {
        'task: loop {
            // Clear any buffered blocks.
            while let Some(known_block) = self.known_blocks.pop(&self.next_yield) {
                debug!(number=self.next_yield, "yielding block");
                self.next_yield += 1;
                yield known_block;
            }

            // Get the tip.
            let block_number = match poll_task.next().await {
                Some(Ok(block_number)) => block_number,
                Some(Err(err)) => {
                    // This is fine.
                    debug!(%err, "block number stream lagged");
                    continue 'task;
                }
                None => {
                    debug!("block number stream ended");
                    break 'task;
                }
            };
            let block_number = block_number.to::<u64>();
            if self.next_yield == NO_BLOCK_NUMBER {
                assert!(block_number < NO_BLOCK_NUMBER, "too many blocks");
                self.next_yield = block_number;
            } else if block_number < self.next_yield {
                debug!(block_number, self.next_yield, "not advanced yet");
                continue 'task;
            }

            // Upgrade the provider.
            let Some(client) = self.client.upgrade() else {
                debug!("client dropped");
                break 'task;
            };

            // Then try to fill as many blocks as possible.
            // TODO: Maybe use `join_all`
            let mut retries = MAX_RETRIES;
            for number in self.next_yield..=block_number {
                debug!(number, "fetching block");
                let block = match client.request("eth_getBlockByNumber", (U64::from(number), false)).await {
                    Ok(Some(block)) => block,
                    Err(RpcError::Transport(err)) if retries > 0 && err.recoverable() => {
                        debug!(number, %err, "failed to fetch block, retrying");
                        retries -= 1;
                        continue;
                    }
                    Ok(None) if retries > 0 => {
                        debug!(number, "failed to fetch block (doesn't exist), retrying");
                        retries -= 1;
                        continue;
                    }
                    Err(err) => {
                        error!(number, %err, "failed to fetch block");
                        break 'task;
                    }
                    Ok(None) => {
                        error!(number, "failed to fetch block (doesn't exist)");
                        break 'task;
                    }
                };
                self.known_blocks.put(number, block);
                if self.known_blocks.len() == BLOCK_CACHE_SIZE.get() {
                    // Cache is full, should be consumed before filling more blocks.
                    debug!(number, "cache full");
                    break;
                }
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
    use alloy_primitives::U256;
    use std::{future::Future, time::Duration};

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    async fn with_timeout<T: Future>(fut: T) -> T::Output {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => panic!("Operation timed out"),
            out = fut => out,
        }
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
        init_tracing();

        let anvil = Anvil::new().spawn();

        let url = if ws { anvil.ws_endpoint() } else { anvil.endpoint() };
        let provider = ProviderBuilder::new().on_builtin(&url).await.unwrap();

        let poller: NewBlocks<_, Ethereum> = NewBlocks::new(provider.weak_client());
        let mut stream = Box::pin(poller.into_stream().await.unwrap());

        // We will also use provider to manipulate anvil instance via RPC.
        provider.anvil_mine(Some(U256::from(1)), None).await.unwrap();

        let block = with_timeout(stream.next()).await.expect("Block wasn't fetched");
        assert!(block.header.number <= 1);
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

        init_tracing();

        let anvil = Anvil::new().spawn();

        let url = if ws { anvil.ws_endpoint() } else { anvil.endpoint() };
        let provider = ProviderBuilder::new().on_builtin(&url).await.unwrap();

        let poller: NewBlocks<_, Ethereum> = NewBlocks::new(provider.weak_client());
        let stream = Box::pin(poller.into_stream().await.unwrap());

        // We will also use provider to manipulate anvil instance via RPC.
        provider.anvil_mine(Some(U256::from(BLOCKS_TO_MINE)), None).await.unwrap();

        let blocks = with_timeout(stream.take(BLOCKS_TO_MINE).collect::<Vec<_>>()).await;
        assert_eq!(blocks.len(), BLOCKS_TO_MINE);
        let first = blocks[0].header.number;
        assert!(first <= 1);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.header.number, first + i as u64);
        }
    }
}
