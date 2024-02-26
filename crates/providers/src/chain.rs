use crate::{new::RootProviderInner, Provider, RootProvider, WeakProvider};
use alloy_network::Network;
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{PollTask, WeakClient};
use alloy_rpc_types::Block;
use alloy_transport::{RpcError, Transport};
use async_stream::stream;
use futures::{Stream, StreamExt};
use lru::LruCache;
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// The size of the block cache.
const BLOCK_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(10) };

/// Maximum number of retries for fetching a block.
const MAX_RETRIES: usize = 3;

pub(crate) struct ChainStreamPoller<P, T: Transport + Clone> {
    provider: WeakProvider<P>,
    poll_task: PollTask<T, (), U64>,
    next_yield: BlockNumber,
    known_blocks: LruCache<BlockNumber, Block>,
}

impl<N: Network, T: Transport + Clone> ChainStreamPoller<RootProviderInner<N, T>, T> {
    pub(crate) fn from_root(p: &RootProvider<N, T>) -> Self {
        let mut this = Self::new(Arc::downgrade(&p.inner), p.inner.weak_client());
        if p.client().is_local() {
            this.poll_task.set_poll_interval(Duration::from_secs(1));
        }
        this
    }
}

impl<P, T: Transport + Clone> ChainStreamPoller<P, T> {
    pub(crate) fn new(provider: WeakProvider<P>, client: WeakClient<T>) -> Self {
        Self {
            provider,
            poll_task: PollTask::new(client, "eth_blockNumber", ()),
            next_yield: BlockNumber::MAX,
            known_blocks: LruCache::new(BLOCK_CACHE_SIZE),
        }
    }

    pub(crate) fn into_stream<N: Network>(mut self) -> impl Stream<Item = Block>
    where
        P: Provider<N, T>,
    {
        stream! {
        let mut poll_task = self.poll_task.spawn().into_stream();
        'task: loop {
            // Clear any buffered blocks.
            while let Some(known_block) = self.known_blocks.pop(&self.next_yield) {
                debug!("yielding block number {}", self.next_yield);
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
            if block_number == self.next_yield {
                continue 'task;
            }
            if self.next_yield > block_number {
                self.next_yield = block_number;
            }

            // Upgrade the provider.
            let Some(provider) = self.provider.upgrade() else {
                debug!("provider dropped");
                break 'task;
            };

            // Then try to fill as many blocks as possible.
            // TODO: Maybe use `join_all`
            let mut retries = MAX_RETRIES;
            for block_number in self.next_yield..=block_number {
                let block = match provider.get_block_by_number(block_number, false).await {
                    Ok(block) => block,
                    Err(RpcError::Transport(err)) if retries > 0 && err.recoverable() => {
                        debug!(block_number, %err, "failed to fetch block, retrying");
                        retries -= 1;
                        continue;
                    }
                    Err(err) => {
                        error!(block_number, %err, "failed to fetch block");
                        break 'task;
                    }
                };
                self.known_blocks.put(block_number, block);
            }
        }
        }
    }
}
