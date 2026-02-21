use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::{RpcError, RpcRecv};
use alloy_network_primitives::HeaderResponse;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientRef, RpcClientInner, WeakClient};
use alloy_transport::TransportResult;
use async_stream::stream;
use futures::Stream;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

pub(super) type RequestFuture<Item> =
    Pin<Box<dyn Future<Output = TransportResult<Item>> + 'static>>;

pub(super) type FutureStepFn<Item> =
    Box<dyn FnMut(Arc<RpcClientInner>, u64, u64) -> (u64, RequestFuture<Item>) + 'static>;

pub(super) fn stream_from_head_futures<Item, HeaderResp>(
    client: WeakClient,
    start_block: u64,
    poll_interval: Duration,
    block_tag: BlockNumberOrTag,
    mut step: FutureStepFn<Item>,
) -> impl Stream<Item = RequestFuture<Item>> + Unpin + 'static
where
    HeaderResp: HeaderResponse + RpcRecv + 'static,
    Item: 'static,
{
    let stream = stream! {
        let mut current_block = start_block;

        'task: loop {
            let Some(client) = client.upgrade() else {
                break 'task;
            };

            let head = match fetch_head_block::<HeaderResp>(client.as_ref(), block_tag).await {
                Ok(head) => head,
                Err(err) => {
                    let fut: RequestFuture<Item> = Box::pin(async move { Err(err) });
                    yield fut;
                    sleep(poll_interval).await;
                    continue 'task;
                }
            };

            if current_block > head {
                sleep(poll_interval).await;
                continue 'task;
            }

            while current_block <= head {
                let (next_block, item_fut) = step(client.clone(), current_block, head);
                if next_block <= current_block {
                    let err = RpcError::local_usage_str(
                        "watch stream step did not advance block cursor",
                    );
                    let fut: RequestFuture<Item> = Box::pin(async move { Err(err) });
                    yield fut;
                    sleep(poll_interval).await;
                    continue 'task;
                }
                current_block = next_block;
                yield item_fut;
            }

            sleep(poll_interval).await;
        }
    };

    Box::pin(stream)
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
