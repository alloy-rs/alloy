use crate::utils;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::RpcError;
use alloy_network::Network;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_rpc_client::WeakClient;
use alloy_transport::TransportResult;
use futures::Stream;
use std::{marker::PhantomData, time::Duration};

use super::watch_from_common::{stream_from_head, StepFn};

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

        let full = kind.is_full();
        let hashes = kind.is_hashes();
        let step: StepFn<N::BlockResponse> = Box::new(move |client, current_block, _head| {
            Box::pin(async move {
                let block = client
                    .request("eth_getBlockByNumber", (BlockNumberOrTag::from(current_block), full))
                    .await?;
                let block = if hashes { utils::convert_to_hashes(block) } else { block };
                let block = block.ok_or(RpcError::NullResp)?;
                Ok((current_block.saturating_add(1), block))
            })
        });

        stream_from_head::<N::BlockResponse, N::HeaderResponse>(
            client,
            start_block,
            poll_interval,
            block_tag,
            step,
        )
    }
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

    #[tokio::test]
    async fn waits_until_head_reaches_start_block() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&0_u64);
        asserter.push_success(&1_u64);
        asserter.push_success(&Some(block(1)));

        let mut stream = provider
            .watch_blocks_from(1)
            .block_tag(BlockNumberOrTag::Latest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 1);
    }

    #[tokio::test]
    async fn fixed_block_tag_number_does_not_fetch_head() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&Some(block(5)));

        let mut stream = provider
            .watch_blocks_from(5)
            .block_tag(BlockNumberOrTag::Number(5))
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 5);
    }

    #[tokio::test]
    async fn earliest_block_tag_starts_at_zero() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        asserter.push_success(&Some(block(0)));

        let mut stream = provider
            .watch_blocks_from(0)
            .block_tag(BlockNumberOrTag::Earliest)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.header.number, 0);
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let provider =
            ProviderBuilder::new().connect_mocked_client(alloy_transport::mock::Asserter::new());
        let mut stream = provider.watch_blocks_from(0).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }
}
