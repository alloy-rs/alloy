use alloy_eips::BlockNumberOrTag;
use alloy_rpc_client::WeakClient;
use alloy_rpc_types_eth::{Filter, Header, Log};
use futures::Stream;
use std::time::Duration;

use super::watch_from_common::{stream_from_head_futures, FutureStepFn, RequestFuture};

const DEFAULT_WINDOW_SIZE: u64 = 1000;
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

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

    /// Converts this builder into a stream of request futures.
    ///
    /// Each future represents one `eth_getLogs` request for a complete window. That means each
    /// buffered in-flight request still covers up to `window_size` blocks (clamped to the head).
    ///
    /// This can be buffered by the caller, for example with
    /// [`StreamExt::buffered`](futures::StreamExt::buffered).
    pub fn into_stream(self) -> impl Stream<Item = RequestFuture<Vec<Log>>> + Unpin + 'static {
        let Self { client, start_block, filter, window_size, poll_interval, block_tag } = self;

        let step: FutureStepFn<Vec<Log>> = Box::new(move |client, current_block, head| {
            let to_block = current_block.saturating_add(window_size - 1).min(head);
            let window_filter = filter.clone().from_block(current_block).to_block(to_block);
            let fut: RequestFuture<Vec<Log>> = Box::pin(async move {
                let logs = client.request("eth_getLogs", (window_filter,)).await?;
                Ok(logs)
            });
            (to_block.saturating_add(1), fut)
        });

        stream_from_head_futures::<Vec<Log>, Header>(
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
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn advances_to_next_window_after_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_success(&11_u64);
        asserter.push_failure_msg("boom");
        asserter.push_success(&12_u64);
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.len(), 1);
    }

    #[tokio::test]
    async fn recovers_after_head_fetch_error() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_failure_msg("head boom");
        asserter.push_success(&10_u64);
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        assert!(first.is_err());

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(second.len(), 1);
    }

    #[tokio::test]
    async fn waits_until_head_reaches_start_block() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_success(&9_u64);
        asserter.push_success(&10_u64);
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);
    }

    #[tokio::test]
    async fn window_size_zero_is_clamped_to_one() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        let no_logs: Vec<Log> = Vec::new();
        asserter.push_success(&11_u64);
        asserter.push_success(&one_log);
        asserter.push_success(&no_logs);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(0)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn fixed_block_tag_number_does_not_fetch_head() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(10))
            .window_size(10)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);
    }

    #[tokio::test]
    async fn stream_ends_when_provider_is_dropped() {
        let provider =
            ProviderBuilder::new().connect_mocked_client(alloy_transport::mock::Asserter::new());
        let mut stream = provider.watch_logs_from(0, &Filter::new()).into_stream();
        drop(provider);

        let next = timeout(Duration::from_secs(1), stream.next()).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn yielded_future_outlives_provider() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        asserter.push_success(&10_u64);
        asserter.push_success(&one_log);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(1)
            .poll_interval(Duration::from_millis(1))
            .into_stream();

        let fut = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        drop(stream);
        drop(provider);

        let logs = timeout(Duration::from_secs(1), fut).await.unwrap().unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn errors_when_cursor_cannot_advance() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);

        let mut stream = provider
            .watch_logs_from(u64::MAX, &Filter::new())
            .block_tag(BlockNumberOrTag::Number(u64::MAX))
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(1);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap();
        let err = first.unwrap_err();
        assert!(err.is_local_usage_error());
    }

    #[tokio::test]
    async fn future_stream_can_be_buffered() {
        let asserter = alloy_transport::mock::Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());

        let one_log: Vec<Log> = vec![Log::default()];
        let no_logs: Vec<Log> = Vec::new();
        asserter.push_success(&13_u64);
        asserter.push_success(&one_log);
        asserter.push_success(&no_logs);

        let mut stream = provider
            .watch_logs_from(10, &Filter::new())
            .block_tag(BlockNumberOrTag::Latest)
            .window_size(2)
            .poll_interval(Duration::from_millis(1))
            .into_stream()
            .buffered(2);

        let first = timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(first.len(), 1);

        let second =
            timeout(Duration::from_secs(1), stream.next()).await.unwrap().unwrap().unwrap();
        assert!(second.is_empty());
    }
}
