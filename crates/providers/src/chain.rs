use crate::{Provider, WeakProvider};
use alloy_network::Network;
use alloy_primitives::BlockNumber;
use alloy_rpc_client::PollTask;
use alloy_rpc_types::Block;
use alloy_transport::{RpcError, Transport};
use async_stream::stream;
use futures::{Stream, StreamExt};
use lru::LruCache;
use std::{num::NonZeroUsize, time::Duration};

/// The size of the block cache.
const BLOCK_CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(10) };

fn chain_stream_poller<P, N, T>(
    provider: WeakProvider<P>,
    from_height: BlockNumber,
    poll_interval: Duration,
) -> impl Stream<Item = Block>
where
    P: Provider<N, T>,
    N: Network,
    T: Transport + Clone,
{
    let mut poll_stream = {
        let upgraded_provider = provider.upgrade().expect("provider dropped before poller started");
        PollTask::new(upgraded_provider.weak_client(), "eth_blockNumber", ())
            .with_poll_interval(poll_interval)
            .spawn()
            .into_stream()
    };

    let mut next_yield = from_height;
    let mut known_blocks = LruCache::<BlockNumber, Block>::new(BLOCK_CACHE_SIZE);

    stream! {
        'task: loop {
            // Clear any buffered blocks.
            while let Some(known_block) = known_blocks.pop(&next_yield) {
                next_yield += 1;
                yield known_block;
            }

            // Get the tip.
            let block_number = match poll_stream.next().await {
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

            // Upgrade the provider.
            let Some(provider) = provider.upgrade() else {
                debug!("provider dropped");
                break 'task;
            };

            // Then try to fill as many blocks as possible.
            // TODO: Maybe use `join_all`
            while !known_blocks.contains(&block_number) {
                let block = provider.get_block_by_number(block_number, false).await;
                match block {
                    Ok(block) => {
                        known_blocks.put(block_number, block);
                    },
                    Err(RpcError::Transport(err)) if err.recoverable() => {
                        continue 'task;
                    },
                    Err(err) => {
                        error!(block_number, %err, "failed to fetch block");
                        break 'task;
                    },
                }
            }
        }
    }
}
