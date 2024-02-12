use std::{num::NonZeroUsize, time::Duration};

use alloy_network::Network;
use alloy_primitives::BlockNumber;
use alloy_rpc_client::PollTask;
use alloy_rpc_types::Block;
use alloy_transport::{RpcError, Transport};
use async_stream::stream;
use futures::{Stream, StreamExt};
use lru::LruCache;

use crate::{Provider, WeakProvider};

/// The size of the block cache.
pub const BLOCK_CACHE_SIZE: NonZeroUsize = match NonZeroUsize::new(10) {
    Some(size) => size,
    None => panic!("BLOCK_CACHE_SIZE must be non-zero"),
};

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
    let mut poll_stream = provider
        .upgrade()
        .map(|provider| {
            PollTask::new((&*provider).weak_client(), "eth_blockNumber", ())
                .with_poll_interval(poll_interval)
                .spawn()
                .into_stream()
        })
        .expect("provider dropped before poller started");

    let mut next_yield = from_height;
    let mut known_blocks: LruCache<BlockNumber, Block> = LruCache::new(BLOCK_CACHE_SIZE);

    stream! {
        'task: loop {
            // first clear any buffered blocks
            if known_blocks.contains(&next_yield) {
                next_yield += 1;
                yield known_blocks.get(&next_yield).unwrap().clone();
                continue;
            }

            let block_number = match poll_stream.next().await {
                Some(Ok(block_number)) => block_number,
                Some(Err(err)) => {
                    tracing::error!(%err, "polling stream lagged");
                    continue;
                },
                None => {
                    tracing::debug!("polling stream ended");
                    break;
                },
            };

            let provider = match provider.upgrade() {
                Some(provider) => provider,
                None => {
                    tracing::debug!("provider dropped");
                    break 'task;
                },
            };

            // Then try to fill as many blocks as possible
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
                        tracing::error!(block_number, %err, "failed to fetch block");
                        break 'task;
                    },
                }
            }

        }
    }
}
