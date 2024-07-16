use alloy_network::{Ethereum, Network};
use alloy_primitives::{BlockNumber, U64};
use alloy_rpc_client::{BatchRequest, PollerBuilder, RpcClientInner, Waiter, WeakClient};
use alloy_rpc_types_eth::Block;
use alloy_transport::Transport;
use async_stream::stream;
use futures::{stream::FuturesOrdered, Stream, StreamExt};
use std::{marker::PhantomData, sync::Arc};
use tokio_stream::wrappers::BroadcastStream;

use super::ChainStreamError;

/// Stream state.
#[derive(Debug)]
pub(super) struct PollerStream<T, N = Ethereum> {
    client: WeakClient<T>,
    /// Next block that should be yielded.
    next_block: BlockNumber,
    /// Last known block number.
    chain_tip: BlockNumber,
    /// Stream of block numbers, updated with respect to polling interval.
    block_numbers_stream: BroadcastStream<U64>,
    _phantom: PhantomData<N>,
}

impl<T: Transport + Clone, N: Network> PollerStream<T, N> {
    pub(super) fn stream(
        client: WeakClient<T>,
        next_yield: Option<u64>,
    ) -> impl Stream<Item = Block> + 'static {
        stream! {
            let mut this = match Self::new(client, next_yield).await {
                Ok(poller) => poller,
                Err(err) => {
                    warn!(%err, "Unable to start the stream");
                    return;
                }
            };
            loop {
                let blocks = match this.new_blocks().await {
                    Ok(blocks) => blocks,
                    Err(err) => {
                        warn!(%err, "irrecoverable error, stopping the stream");
                        break;
                    }
                };
                for block in blocks {
                    // The implementation is stateful, and we assume that `PollerStream`
                    // will only provide us with blocks that _should_ be yielded.
                    // Any "unexpected" situations like reorgs should be handled by the
                    // `PollerStream::new_blocks` itself.
                    // The stream is only responsible for yielding blocks provided by the
                    // poller.
                    yield block;
                    this.next_block += 1;
                }
            }
        }
    }

    async fn new(
        weak_client: WeakClient<T>,
        next_yield: Option<BlockNumber>,
    ) -> Result<Self, ChainStreamError<T>> {
        let client = Self::client(&weak_client)?;
        let chain_tip = client
            .request("eth_blockNumber", ())
            .await
            .map(|b: U64| b.to::<u64>())
            .map_err(|e| ChainStreamError::Rpc(e))?;
        let next_yield = next_yield.unwrap_or(chain_tip);

        let block_numbers_stream = PollerBuilder::new(weak_client.clone(), "eth_blockNumber", ())
            .spawn()
            .into_stream_raw();
        Ok(Self {
            client: weak_client,
            next_block: next_yield,
            chain_tip,
            block_numbers_stream,
            _phantom: PhantomData,
        })
    }

    fn client(client: &WeakClient<T>) -> Result<Arc<RpcClientInner<T>>, ChainStreamError<T>> {
        client.upgrade().ok_or(ChainStreamError::ClientDropped)
    }

    /// Updates the chain tip so that it is greater than `next_yield`.
    async fn update_chain_tip(&mut self) -> Result<(), ChainStreamError<T>> {
        // Loop until chain tip is advanced.
        while self.next_block > self.chain_tip {
            let new_number = match self.block_numbers_stream.next().await {
                Some(Ok(block_number)) => block_number.to::<u64>(),
                Some(Err(err)) => {
                    // This is fine.
                    debug!(%err, "polling stream lagged");
                    continue;
                }
                None => {
                    debug!("polling stream ended");
                    return Err(ChainStreamError::PollingStreamEnded);
                }
            };
            self.chain_tip = new_number;
        }
        Ok(())
    }

    /// Tries to fetch the range of blocks that can surely be yielded via single batch request.
    async fn fetch_blocks(&self) -> Result<Vec<Block>, ChainStreamError<T>> {
        // We don't want to request too many batches by accident.
        // TODO: Should be a property on transport or client.
        const MAX_BATCH_SIZE: usize = 10;

        let range_end = self.chain_tip.min(self.next_block + MAX_BATCH_SIZE as u64 - 1);
        let range = self.next_block..=range_end;
        if range.is_empty() {
            return Ok(Vec::new());
        }

        // Perform a batch request.
        let client = Self::client(&self.client)?;
        let mut batch_request = BatchRequest::new(client.as_ref());
        // Each request in the batch has its own future.
        let futures: FuturesOrdered<Waiter<Option<Block>>> = range
            .map(|number| {
                batch_request
                    .add_call::<_, Option<Block>>(
                        "eth_getBlockByNumber",
                        &(U64::from(number), false),
                    )
                    .expect("Cannot serialize call params")
            })
            .collect();
        batch_request.send().await.map_err(ChainStreamError::Rpc)?;

        // We expect all the futures to complete simultaneously, given that it's a batch request,
        // so we will collect them and let the caller yield the results.
        let mut blocks = Vec::with_capacity(futures.len());
        for maybe_block in futures.collect::<Vec<_>>().await {
            match maybe_block {
                Ok(Some(block)) => {
                    debug!(number = self.next_block, "yielding block");
                    blocks.push(block)
                }
                Ok(None) => {
                    // We expect that the remaining blocks will be missing too.
                    break;
                }
                Err(err) => {
                    error!(%err, "failed to fetch blocks");
                    return Err(ChainStreamError::Rpc(err));
                }
            }
        }
        Ok(blocks)
    }

    /// Performs a single step of the polling loop.
    /// Either returns a list of blocks to yield, or an irrecoverable error.
    async fn new_blocks(&mut self) -> Result<Vec<Block>, ChainStreamError<T>> {
        self.update_chain_tip().await?;
        self.fetch_blocks().await
    }
}
