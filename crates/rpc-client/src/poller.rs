use crate::WeakClient;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_transport::utils::Spawnable;
use async_stream::stream;
use futures::{Stream, StreamExt};
use serde::Serialize;
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing_futures::Instrument;

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
    pub fn set_channel_size(&mut self, channel_size: usize) {
        self.channel_size = channel_size;
    }

    /// Sets the channel size for the poller task.
    pub fn with_channel_size(mut self, channel_size: usize) -> Self {
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
    pub fn set_poll_interval(&mut self, poll_interval: Duration) {
        self.poll_interval = poll_interval;
    }

    /// Sets the duration between polls.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
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
    pub fn into_stream(self) -> impl Stream<Item = Resp> + Unpin {
        Box::pin(self.into_local_stream())
    }

    fn into_local_stream(self) -> impl Stream<Item = Resp> {
        let span = debug_span!("poller", method = %self.method);
        stream! {
        let mut params = ParamsOnce::Typed(self.params);
        for _ in 0..self.limit {
            let Some(client) = self.client.upgrade() else {
                debug!("client dropped");
                break;
            };

            // Avoid serializing the params more than once.
            let params = match params.get() {
                Ok(p) => p,
                Err(err) => {
                    error!(%err, "failed to serialize params");
                    break;
                }
            };

            trace!("polling");
            match client.request(self.method.clone(), params).await {
                Ok(resp) => yield resp,
                Err(err) => {
                    error!(%err, "failed to poll");
                }
            }

            trace!(duration=?self.poll_interval, "sleeping");
            sleep(self.poll_interval).await;
        }
        }
        .instrument(span)
    }

    /// Returns the [`WeakClient`] associated with the poller.
    pub fn client(&self) -> WeakClient {
        self.client.clone()
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

// Serializes the parameters only once.
enum ParamsOnce<P> {
    Typed(P),
    Serialized(Box<RawValue>),
}

impl<P: Serialize> ParamsOnce<P> {
    #[inline]
    fn get(&mut self) -> serde_json::Result<&RawValue> {
        match self {
            Self::Typed(_) => self.init(),
            Self::Serialized(p) => Ok(p),
        }
    }

    #[cold]
    fn init(&mut self) -> serde_json::Result<&RawValue> {
        let Self::Typed(p) = self else { unreachable!() };
        let v = serde_json::value::to_raw_value(p)?;
        *self = Self::Serialized(v);
        let Self::Serialized(v) = self else { unreachable!() };
        Ok(v)
    }
}

#[cfg(test)]
#[allow(clippy::missing_const_for_fn)]
fn _assert_unpin() {
    fn _assert<T: Unpin>() {}
    _assert::<PollChannel<()>>();
}
