use crate::WeakClient;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_transport::utils::Spawnable;
use futures::{future::BoxFuture, ready, Future, FutureExt, Stream, StreamExt};
use serde::Serialize;
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::Span;

#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

/// A poller task builder.
///
/// This builder is used to create a poller task that repeatedly polls a method on a client and
/// sends the responses to a channel. By default, this is done every 10 seconds, with a channel size
/// of 16, and no limit on the number of successful polls. This is all configurable.
///
/// The builder is consumed using the [`spawn`](Self::spawn) method, which returns a channel to
/// receive the responses. The task will continue to poll until either the client or the channel is
/// dropped.
///
/// The channel can be converted into a stream using the [`into_stream`](PollChannel::into_stream)
/// method.
///
/// Alternatively, [`into_stream`](Self::into_stream) on the builder can be used to directly return
/// a stream of responses on the current thread, instead of spawning a task.
///
/// # Examples
///
/// Poll `eth_blockNumber` every 5 seconds:
///
/// ```no_run
/// # async fn example(client: alloy_rpc_client::RpcClient) -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_primitives::U64;
/// use alloy_rpc_client::PollerBuilder;
/// use futures_util::StreamExt;
///
/// let poller: PollerBuilder<alloy_rpc_client::NoParams, U64> = client
///     .prepare_static_poller("eth_blockNumber", [])
///     .with_poll_interval(std::time::Duration::from_secs(5));
/// let mut stream = poller.into_stream();
/// while let Some(block_number) = stream.next().await {
///    println!("polled block number: {block_number}");
/// }
/// # Ok(())
/// # }
/// ```
// TODO: make this be able to be spawned on the current thread instead of forcing a task.
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `spawn` or `into_stream`"]
pub struct PollerBuilder<Params, Resp> {
    /// The client to poll with.
    client: WeakClient,

    /// Request Method
    method: Cow<'static, str>,
    params: Params,

    // config options
    channel_size: usize,
    poll_interval: Duration,
    limit: usize,

    _pd: PhantomData<fn() -> Resp>,
}

impl<Params, Resp> PollerBuilder<Params, Resp>
where
    Params: RpcSend + 'static,
    Resp: RpcRecv + Clone,
{
    /// Create a new poller task.
    pub fn new(client: WeakClient, method: impl Into<Cow<'static, str>>, params: Params) -> Self {
        let poll_interval =
            client.upgrade().map_or_else(|| Duration::from_secs(7), |c| c.poll_interval());
        Self {
            client,
            method: method.into(),
            params,
            channel_size: 16,
            poll_interval,
            limit: usize::MAX,
            _pd: PhantomData,
        }
    }

    /// Returns the channel size for the poller task.
    pub const fn channel_size(&self) -> usize {
        self.channel_size
    }

    /// Sets the channel size for the poller task.
    pub const fn set_channel_size(&mut self, channel_size: usize) {
        self.channel_size = channel_size;
    }

    /// Sets the channel size for the poller task.
    pub const fn with_channel_size(mut self, channel_size: usize) -> Self {
        self.set_channel_size(channel_size);
        self
    }

    /// Returns the limit on the number of successful polls.
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Sets a limit on the number of successful polls.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit.unwrap_or(usize::MAX);
    }

    /// Sets a limit on the number of successful polls.
    pub fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.set_limit(limit);
        self
    }

    /// Returns the duration between polls.
    pub const fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// Sets the duration between polls.
    pub const fn set_poll_interval(&mut self, poll_interval: Duration) {
        self.poll_interval = poll_interval;
    }

    /// Sets the duration between polls.
    pub const fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.set_poll_interval(poll_interval);
        self
    }

    /// Starts the poller in a new task, returning a channel to receive the responses on.
    pub fn spawn(self) -> PollChannel<Resp> {
        let (tx, rx) = broadcast::channel(self.channel_size);
        self.into_future(tx).spawn_task();
        rx.into()
    }

    async fn into_future(self, tx: broadcast::Sender<Resp>) {
        let mut stream = self.into_stream();
        while let Some(resp) = stream.next().await {
            if tx.send(resp).is_err() {
                debug!("channel closed");
                break;
            }
        }
    }

    /// Starts the poller and returns the stream of responses.
    ///
    /// Note that this does not spawn the poller on a separate task, thus all responses will be
    /// polled on the current thread once this stream is polled.
    pub fn into_stream(self) -> PollerStream<Resp> {
        PollerStream::new(self)
    }

    /// Returns the [`WeakClient`] associated with the poller.
    pub fn client(&self) -> WeakClient {
        self.client.clone()
    }
}

/// State for the polling stream.
enum PollState<Resp> {
    /// Waiting to start the next poll.
    Waiting,
    /// Currently polling for a response.
    Polling(
        BoxFuture<
            'static,
            Result<Resp, alloy_transport::RpcError<alloy_transport::TransportErrorKind>>,
        >,
    ),
    /// Sleeping between polls.
    Sleeping(Pin<Box<tokio::time::Sleep>>),
}

/// A stream of responses from polling an RPC method.
///
/// This stream polls the given RPC method at the specified interval and yields the responses.
///
/// # Examples
///
/// ```no_run
/// # async fn example(client: alloy_rpc_client::RpcClient) -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_primitives::U64;
/// use futures_util::StreamExt;
///
/// // Create a poller that fetches block numbers
/// let poller = client
///     .prepare_static_poller("eth_blockNumber", [])
///     .with_poll_interval(std::time::Duration::from_secs(1));
///
/// // Convert the block number to a more useful format
/// let mut stream = poller.into_stream().map(|block_num: U64| block_num.to::<u64>());
///
/// while let Some(block_number) = stream.next().await {
///     println!("Current block: {}", block_number);
/// }
/// # Ok(())
/// # }
/// ```
pub struct PollerStream<Resp, Output = Resp, Map = fn(Resp) -> Output> {
    client: WeakClient,
    method: Cow<'static, str>,
    params: Box<RawValue>,
    poll_interval: Duration,
    limit: usize,
    poll_count: usize,
    state: PollState<Resp>,
    span: Span,
    map: Map,
    _pd: PhantomData<fn() -> Output>,
}

impl<Resp, Output, Map> std::fmt::Debug for PollerStream<Resp, Output, Map> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerStream")
            .field("method", &self.method)
            .field("poll_interval", &self.poll_interval)
            .field("limit", &self.limit)
            .field("poll_count", &self.poll_count)
            .finish_non_exhaustive()
    }
}

impl<Resp> PollerStream<Resp> {
    fn new<Params: Serialize>(builder: PollerBuilder<Params, Resp>) -> Self {
        let span = debug_span!("poller", method = %builder.method);

        // Serialize params once
        let params = serde_json::value::to_raw_value(&builder.params).unwrap_or_else(|err| {
            error!(%err, "failed to serialize params during initialization");
            // Return empty params, stream will terminate on first poll
            Box::<RawValue>::default()
        });

        Self {
            client: builder.client,
            method: builder.method,
            params,
            poll_interval: builder.poll_interval,
            limit: builder.limit,
            poll_count: 0,
            state: PollState::Waiting,
            span,
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<Resp, Output, Map> PollerStream<Resp, Output, Map>
where
    Map: Fn(Resp) -> Output,
{
    /// Maps the responses using the provided function.
    pub fn map<NewOutput, NewMap>(self, map: NewMap) -> PollerStream<Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        PollerStream {
            client: self.client,
            method: self.method,
            params: self.params,
            poll_interval: self.poll_interval,
            limit: self.limit,
            poll_count: self.poll_count,
            state: self.state,
            span: self.span,
            map,
            _pd: PhantomData,
        }
    }
}

impl<Resp, Output, Map> Stream for PollerStream<Resp, Output, Map>
where
    Resp: RpcRecv + 'static,
    Map: Fn(Resp) -> Output + Unpin,
{
    type Item = Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let _guard = this.span.enter();

        loop {
            match &mut this.state {
                PollState::Waiting => {
                    // Check if we've reached the limit
                    if this.poll_count >= this.limit {
                        debug!("poll limit reached");
                        return Poll::Ready(None);
                    }

                    // Check if client is still alive
                    let Some(client) = this.client.upgrade() else {
                        debug!("client dropped");
                        return Poll::Ready(None);
                    };

                    // Start polling
                    trace!("polling");
                    let method = this.method.clone();
                    let params = this.params.clone();
                    let fut = Box::pin(async move { client.request(method, params).await });
                    this.state = PollState::Polling(fut);
                }
                PollState::Polling(fut) => {
                    match ready!(fut.poll_unpin(cx)) {
                        Ok(resp) => {
                            this.poll_count += 1;
                            // Start sleeping before next poll
                            trace!(duration=?this.poll_interval, "sleeping");
                            let sleep = Box::pin(sleep(this.poll_interval));
                            this.state = PollState::Sleeping(sleep);
                            return Poll::Ready(Some((this.map)(resp)));
                        }
                        Err(err) => {
                            error!(%err, "failed to poll");
                            // Start sleeping before retry
                            trace!(duration=?this.poll_interval, "sleeping after error");
                            let sleep = Box::pin(sleep(this.poll_interval));
                            this.state = PollState::Sleeping(sleep);
                        }
                    }
                }
                PollState::Sleeping(sleep) => {
                    ready!(sleep.as_mut().poll(cx));
                    this.state = PollState::Waiting;
                }
            }
        }
    }
}

/// A channel yielding responses from a poller task.
///
/// This stream is backed by a coroutine, and will continue to produce responses
/// until the poller task is dropped. The poller task is dropped when all
/// [`RpcClient`] instances are dropped, or when all listening `PollChannel` are
/// dropped.
///
/// The poller task also ignores errors from the server and deserialization
/// errors, and will continue to poll until the client is dropped.
///
/// [`RpcClient`]: crate::RpcClient
#[derive(Debug)]
pub struct PollChannel<Resp> {
    rx: broadcast::Receiver<Resp>,
}

impl<Resp> From<broadcast::Receiver<Resp>> for PollChannel<Resp> {
    fn from(rx: broadcast::Receiver<Resp>) -> Self {
        Self { rx }
    }
}

impl<Resp> Deref for PollChannel<Resp> {
    type Target = broadcast::Receiver<Resp>;

    fn deref(&self) -> &Self::Target {
        &self.rx
    }
}

impl<Resp> DerefMut for PollChannel<Resp> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rx
    }
}

impl<Resp> PollChannel<Resp>
where
    Resp: RpcRecv + Clone,
{
    /// Resubscribe to the poller task.
    pub fn resubscribe(&self) -> Self {
        Self { rx: self.rx.resubscribe() }
    }

    /// Converts the poll channel into a stream.
    // TODO: can we name this type?
    pub fn into_stream(self) -> impl Stream<Item = Resp> + Unpin {
        self.into_stream_raw().filter_map(|r| futures::future::ready(r.ok()))
    }

    /// Converts the poll channel into a stream that also yields
    /// [lag errors](tokio_stream::wrappers::errors::BroadcastStreamRecvError).
    pub fn into_stream_raw(self) -> BroadcastStream<Resp> {
        self.rx.into()
    }
}

#[cfg(test)]
#[allow(clippy::missing_const_for_fn)]
fn _assert_unpin() {
    fn _assert<T: Unpin>() {}
    _assert::<PollChannel<()>>();
}
