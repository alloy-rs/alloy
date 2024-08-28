use alloy_network::{Ethereum, Network};
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{NoParams, PollerBuilder, WeakClient};
use alloy_rpc_types_eth::Block;
use alloy_transport::{RpcError, Transport};
use async_stream::stream;
use futures::{Stream, StreamExt};
use lru::LruCache;
use std::{marker::PhantomData, num::NonZeroUsize};

/// The size of the block cache.
const BLOCK_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(10) };

/// Maximum number of retries for fetching a block.
const MAX_RETRIES: usize = 3;

/// Default block number for when we don't have a block yet.
const NO_BLOCK_NUMBER: BlockNumber = BlockNumber::MAX;

pub(crate) struct ChainStreamPoller<T, N = Ethereum> {
    client: WeakClient<T>,
    poll_task: PollerBuilder<T, NoParams, U64>,
    next_yield: BlockNumber,
    known_blocks: LruCache<BlockNumber, Block>,
    _phantom: PhantomData<N>,
}

impl<T: Transport + Clone, N: Network> ChainStreamPoller<T, N> {
    pub(crate) fn from_weak_client(w: WeakClient<T>) -> Self {
        Self::new(w)
    }

    pub(crate) fn new(client: WeakClient<T>) -> Self {
        Self::with_next_yield(client, NO_BLOCK_NUMBER)
    }

    /// Can be used to force the poller to start at a specific block number.
    /// Mostly useful for tests.
    fn with_next_yield(client: WeakClient<T>, next_yield: BlockNumber) -> Self {
        Self {
            client: client.clone(),
            poll_task: PollerBuilder::new(client, "eth_blockNumber", []),
            next_yield,
            known_blocks: LruCache::new(BLOCK_CACHE_SIZE),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn into_stream(mut self) -> impl Stream<Item = Block> + 'static {
        stream! {
        let mut poll_task = self.poll_task.spawn().into_stream_raw();
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
                    debug!(%err, "polling stream lagged");
                    continue 'task;
                }
                None => {
                    debug!("polling stream ended");
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
    use std::{future::Future, time::Duration};

    use crate::{ext::AnvilApi, ProviderBuilder};
    use alloy_node_bindings::Anvil;
    use alloy_primitives::U256;
    use alloy_rpc_client::ReqwestClient;

    use super::*;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    async fn with_timeout<T: Future>(fut: T) -> T::Output {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("Operation timed out"),
            out = fut => out,
        }
    }

    #[tokio::test]
    async fn yield_block() {
        init_tracing();

        let anvil = Anvil::new().spawn();

        let client = ReqwestClient::new_http(anvil.endpoint_url());
        let poller: ChainStreamPoller<_, Ethereum> =
            ChainStreamPoller::with_next_yield(client.get_weak(), 1);
        let mut stream = Box::pin(poller.into_stream());

        // We will also use provider to manipulate anvil instance via RPC.
        let provider = ProviderBuilder::new().on_http(anvil.endpoint_url());
        provider.anvil_mine(Some(U256::from(1)), None).await.unwrap();

        let block = with_timeout(stream.next()).await.expect("Block wasn't fetched");
        assert_eq!(block.header.number, 1);
    }

    #[tokio::test]
    async fn yield_many_blocks() {
        // Make sure that we can process more blocks than fits in the cache.
        const BLOCKS_TO_MINE: usize = BLOCK_CACHE_SIZE.get() + 1;

        init_tracing();

        let anvil = Anvil::new().spawn();

        let client = ReqwestClient::new_http(anvil.endpoint_url());
        let poller: ChainStreamPoller<_, Ethereum> =
            ChainStreamPoller::with_next_yield(client.get_weak(), 1);
        let stream = Box::pin(poller.into_stream());

        // We will also use provider to manipulate anvil instance via RPC.
        let provider = ProviderBuilder::new().on_http(anvil.endpoint_url());
        provider.anvil_mine(Some(U256::from(BLOCKS_TO_MINE)), None).await.unwrap();

        let blocks = with_timeout(stream.take(BLOCKS_TO_MINE).collect::<Vec<_>>()).await;
        assert_eq!(blocks.len(), BLOCKS_TO_MINE);
    }
}
