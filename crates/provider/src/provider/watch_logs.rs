use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::{RpcError, RpcRecv};
use alloy_network_primitives::HeaderResponse;
use alloy_primitives::U64;
use alloy_rpc_client::{ClientRef, WeakClient};
use alloy_rpc_types_eth::{Filter, Header, Log};
use alloy_transport::TransportResult;
use async_stream::stream;
use futures::Stream;
use std::time::Duration;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

const DEFAULT_WINDOW_SIZE: u64 = 2_000;
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(7);

/// A builder for streaming logs from a historical block and continuing indefinitely.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchLogsFrom {
    client: WeakClient,
    start_block: u64,
    filter: Filter,
    window_size: u64,
    poll_interval: Duration,
    block_tag: BlockNumberOrTag,
}

impl WatchLogsFrom {
    /// Creates a new [`WatchLogsFrom`] builder.
    pub(crate) const fn new(client: WeakClient, start_block: u64, filter: Filter) -> Self {
        Self {
            client,
            start_block,
            filter,
            window_size: DEFAULT_WINDOW_SIZE,
            poll_interval: DEFAULT_POLL_INTERVAL,
            block_tag: BlockNumberOrTag::Finalized,
        }
    }

    /// Sets the number of blocks included in each `eth_getLogs` request.
    pub const fn window_size(mut self, window_size: u64) -> Self {
        self.window_size = if window_size == 0 { 1 } else { window_size };
        self
    }

    /// Sets the poll interval used when the stream is caught up.
    pub const fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Sets the head block tag used to determine stream progress.
    pub const fn block_tag(mut self, block_tag: BlockNumberOrTag) -> Self {
        self.block_tag = block_tag;
        self
    }

    /// Converts this builder into a stream of log windows.
    pub fn into_stream(self) -> impl Stream<Item = TransportResult<Vec<Log>>> + Unpin + 'static {
        let Self { client, start_block, filter, window_size, poll_interval, block_tag } = self;

        let stream = stream! {
            let mut current_block = start_block;

            'task: loop {
                let Some(client) = client.upgrade() else {
                    break 'task;
                };

                let head = match fetch_head_block::<Header>(client.as_ref(), block_tag).await {
                    Ok(head) => head,
                    Err(err) => {
                        yield Err(err);
                        sleep(poll_interval).await;
                        continue 'task;
                    }
                };

                if current_block > head {
                    sleep(poll_interval).await;
                    continue 'task;
                }

                while current_block <= head {
                    let to_block =
                        current_block.saturating_add(window_size - 1).min(head);
                    let window_filter =
                        filter.clone().from_block(current_block).to_block(to_block);

                    match fetch_logs(client.as_ref(), &window_filter).await {
                        Ok(logs) => {
                            current_block = to_block.saturating_add(1);
                            yield Ok(logs);
                        }
                        Err(err) => {
                            yield Err(err);
                            sleep(poll_interval).await;
                            continue 'task;
                        }
                    }
                }

                sleep(poll_interval).await;
            }
        };

        Box::pin(stream)
    }
}

pub(super) async fn fetch_head_block<HeaderResp: HeaderResponse + RpcRecv>(
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

async fn fetch_logs(client: ClientRef<'_>, filter: &Filter) -> TransportResult<Vec<Log>> {
    client.request("eth_getLogs", (filter,)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};
    use futures::StreamExt;
    use tokio::time::timeout;

    #[tokio::test]
    async fn streams_log_windows() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        let no_logs: Vec<Log> = Vec::new();
        asserter.push_success(&12_u64);
        asserter.push_success(&one_log);
        asserter.push_success(&no_logs);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn retries_same_window_after_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_success(&11_u64);
        asserter.push_failure_msg("boom");
        asserter.push_success(&11_u64);
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.len(), 1);
    }
}
