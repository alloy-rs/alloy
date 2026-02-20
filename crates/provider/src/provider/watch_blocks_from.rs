use crate::utils;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::RpcError;
use alloy_network::Network;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_rpc_client::{ClientRef, WeakClient};
use alloy_transport::TransportResult;
use async_stream::stream;
use futures::Stream;
use std::{marker::PhantomData, time::Duration};

use super::watch_logs::fetch_head_block;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(7);

/// A builder for streaming blocks from a historical block and continuing indefinitely.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchBlocksFrom<N: Network> {
    client: WeakClient,
    start_block: u64,
    poll_interval: Duration,
    block_tag: BlockNumberOrTag,
    kind: BlockTransactionsKind,
    _phantom: PhantomData<N>,
}

impl<N: Network> WatchBlocksFrom<N> {
    /// Creates a new [`WatchBlocksFrom`] builder.
    pub(crate) const fn new(client: WeakClient, start_block: u64) -> Self {
        Self {
            client,
            start_block,
            poll_interval: DEFAULT_POLL_INTERVAL,
            block_tag: BlockNumberOrTag::Finalized,
            kind: BlockTransactionsKind::Hashes,
            _phantom: PhantomData,
        }
    }

    /// Streams blocks with full transaction bodies.
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Streams blocks with transaction hashes only.
    pub const fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
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

    /// Converts this builder into a stream of blocks.
    pub fn into_stream(
        self,
    ) -> impl Stream<Item = TransportResult<N::BlockResponse>> + Unpin + 'static {
        let Self { client, start_block, poll_interval, block_tag, kind, _phantom } = self;

        let stream = stream! {
            let mut current_block = start_block;

            'task: loop {
                let Some(client) = client.upgrade() else {
                    break 'task;
                };

                let head = match fetch_head_block::<N::HeaderResponse>(client.as_ref(), block_tag).await {
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
                    let block = match fetch_block::<N>(
                        client.as_ref(),
                        current_block,
                        kind.is_full(),
                    )
                    .await
                    {
                        Ok(Some(block)) => Some(block),
                        Ok(None) => {
                            yield Err(RpcError::NullResp);
                            sleep(poll_interval).await;
                            continue 'task;
                        }
                        Err(err) => {
                            yield Err(err);
                            sleep(poll_interval).await;
                            continue 'task;
                        }
                    };

                    let block = if kind.is_hashes() {
                        utils::convert_to_hashes(block)
                    } else {
                        block
                    };

                    let Some(block) = block else {
                        yield Err(RpcError::NullResp);
                        sleep(poll_interval).await;
                        continue 'task;
                    };

                    current_block = current_block.saturating_add(1);
                    yield Ok(block);
                }

                sleep(poll_interval).await;
            }
        };

        Box::pin(stream)
    }
}

async fn fetch_block<N: Network>(
    client: ClientRef<'_>,
    number: u64,
    full: bool,
) -> TransportResult<Option<N::BlockResponse>> {
    client.request("eth_getBlockByNumber", (BlockNumberOrTag::from(number), full)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::Block;
    use alloy_transport::{
        layers::{RetryBackoffLayer, RetryPolicy},
        mock::MockTransport,
    };
    use futures::StreamExt;
    use tokio::time::timeout;

    fn block(number: u64) -> Block {
        let mut block: Block = Block::default();
        block.header.inner.number = number;
        block
    }

    #[tokio::test]
    async fn streams_blocks_from_start_block() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&3_u64);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&Some(block(2)));
        asserter.push_success(&Some(block(3)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 2);

        let third = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(third.header.number, 3);
    }

    #[tokio::test]
    async fn retries_same_block_after_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&2_u64);
        asserter.push_failure_msg("boom");
        asserter.push_success(&2_u64);
        asserter.push_success(&Some(block(1)));
        asserter.push_success(&Some(block(2)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.header.number, 1);

        let third = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(third.header.number, 2);
    }

    #[tokio::test]
    async fn uses_provider_retry_layer() {
        #[derive(Clone, Debug)]
        struct AlwaysRetryPolicy;

        impl RetryPolicy for AlwaysRetryPolicy {
            fn should_retry(&self, _error: &alloy_transport::TransportError) -> bool {
                true
            }

            fn backoff_hint(&self, _error: &alloy_transport::TransportError) -> Option<Duration> {
                None
            }
        }

        let asserter = alloy_transport::mock::Asserter::new();
        let retry_layer = RetryBackoffLayer::new_with_policy(3, 0, 10_000, AlwaysRetryPolicy);
        let client = RpcClient::builder()
            .layer(retry_layer)
            .transport(MockTransport::new(asserter.clone()), true);
        let provider = ProviderBuilder::new().connect_client(client);

        asserter.push_failure_msg("temporary head error");
        asserter.push_success(&1_u64);
        asserter.push_failure_msg("temporary block error");
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);
    }
}
