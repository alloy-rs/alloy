use crate::{provider::watch_from_common::FixedBuf, utils, WatchBlocksFrom};
use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumberOrTag;
use alloy_network::{BlockResponse as _, Network};
use alloy_network_primitives::{BlockTransactionsKind, HeaderResponse};
use alloy_rpc_client::WeakClient;
use alloy_transport::TransportResult;
use async_stream::{stream, try_stream};
use futures::{Stream, StreamExt as _};
use std::{collections::VecDeque, marker::PhantomData, pin::pin, time::Duration};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

use super::watch_from_common::{stream_from_head_futures, FutureStepFn, RequestFuture};
use crate::transport::TransportErrorKind;

const RPC_CONCURRENCY_DEFAULT: usize = 4;
const MAX_REORG_DEPTH_DEFAULT: u64 = 64;

/// A builder for streaming blocks from a historical block and continuing indefinitely.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchCanonicalBlocksFrom<N: Network> {
    watch_blocks_from: WatchBlocksFrom<N>,
    rpc_concurrency: usize,
    max_reorg_depth: usize,
}

#[derive(Debug, Clone)]
pub enum CanonicalItem<T> {
    Added(T),
    Removed(T),
}

impl<N: Network> WatchCanonicalBlocksFrom<N> {
    pub fn into_stream(
        self,
    ) -> impl Stream<Item = TransportResult<CanonicalItem<N::BlockResponse>>> + Unpin + 'static
    {
        stream! {

            let mut buffer = FixedBuf::new(self.max_reorg_depth);

            loop {
                let mut stream = self.watch_blocks_from.clone().into_stream().buffered(self.rpc_concurrency).peekable();
                loop {
                    let block = stream.peek().await.expect("Stream never ends")?;

                    loop {
                        if let Some(expected_parent_hash) = buffer.last().map(|b: &N::BlockResponse| b.header().parent_hash()) {
                            let parent_hash = block.header().parent_hash();
                            if parent_hash != expected_parent_hash {
                                // Reorg detected.
                                // first step is to check if the parent exists in our buffer.

                                if let Some(pos) = buffer.iter().rev().position(|b| b.header().hash() == parent_hash) {
                                    // we found the parent in our buffer, so we can pop until we get to it.
                                    for _ in 0..pos {
                                        let old = buffer.pop().expect("position is always < buffer.len()");
                                        yield Ok(CanonicalItem::Removed(old));
                                    }
                                    // We can now break out of the loop and add the new block.
                                    break;
                                } else {
                                    // Parent was not found in buffer.
                                    // Request the parent manually
                                    let parent = self.watch_blocks_from.get_block(block.header().number() - 1).await?;

                                }


                                // TODO: handle subtractions
                                let parent = self.watch_blocks_from.get_block(block.header().number() - 1).await?;
                                
                                // match buffer.pop() {
                                //     Some(old) => {
                                //         // pop the last item and try again until we find a common ancestor or run out of history
                                //         yield Ok(CanonicalItem::Removed(old)); 
                                //
                                //     },
                                //     None => {
                                //         // We are in a reorg but have no history
                                //         // TODO: halt stream here.
                                //         yield Err(TransportErrorKind::custom_str("reorg detected but no history in buffer to pop"))
                                //     }
                                // }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    buffer.push(block.clone());
                    yield Ok(CanonicalItem::Added(block)); 
                }
                // let Some(client) = client.upgrade() else {
                //     break 'task;
                // };
                //
                // let head = match fetch_head_block::<HeaderResp>(client.as_ref(), block_tag).await {
                //     Ok(head) => head,
                //     Err(err) => {
                //         let fut: RequestFuture<Item> = Box::pin(async move { Err(err) });
                //         yield fut;
                //         sleep(poll_interval).await;
                //         continue 'task;
                //     }
                // };
                //
                // if current_block > head {
                //     sleep(poll_interval).await;
                //     continue 'task;
                // }
                //
                // while current_block <= head {
                //     let (next_block, item_fut) = step(client.clone(), current_block, head);
                //     if next_block <= current_block {
                //         let err = RpcError::local_usage_str(
                //             "watch stream step did not advance block cursor",
                //         );
                //         let fut: RequestFuture<Item> = Box::pin(async move { Err(err) });
                //         yield fut;
                //         sleep(poll_interval).await;
                //         continue 'task;
                //     }
                //     current_block = next_block;
                //     yield item_fut;
                // }
                //
                // sleep(poll_interval).await;
            }

        }
        .boxed::<>()
    }
}
