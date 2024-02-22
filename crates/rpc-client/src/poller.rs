use crate::WeakClient;
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_transport::{utils::Spawnable, Transport};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// A Poller task.
#[derive(Debug)]
pub struct PollTask<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    /// The client to poll with.
    client: WeakClient<Conn>,

    /// Request Method
    method: &'static str,
    params: Params,

    // config options
    channel_size: usize,
    poll_interval: Duration,
    limit: usize,

    _pd: PhantomData<fn() -> Resp>,
}

impl<Conn, Params, Resp> PollTask<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam + 'static,
    Resp: RpcReturn + Clone,
{
    /// Create a new poller task with cloneable params.
    pub fn new(client: WeakClient<Conn>, method: &'static str, params: Params) -> Self {
        Self {
            client,
            method,
            params,
            channel_size: 16,
            poll_interval: Duration::from_secs(10),
            limit: usize::MAX,
            _pd: PhantomData,
        }
    }

    /// Returns the channel size for the poller task.
    pub fn channel_size(&self) -> usize {
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

    /// Retuns the limit on the number of succesful polls.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Sets a limit on the number of succesful polls.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit.unwrap_or(usize::MAX);
    }

    /// Sets a limit on the number of succesful polls.
    pub fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.set_limit(limit);
        self
    }

    /// Returns the duration between polls.
    pub fn poll_interval(&self) -> Duration {
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

    /// Spawn the poller task, producing a stream of responses.
    pub fn spawn(self) -> PollChannel<Resp> {
        let (tx, rx) = broadcast::channel(self.channel_size);
        let fut = async move {
            for _ in 0..self.limit {
                let Some(client) = self.client.upgrade() else {
                    break;
                };
                match client.prepare(self.method, &self.params).await {
                    Ok(resp) => {
                        if tx.send(resp).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!(%e, "error response in polling request");
                    }
                }
                tokio::time::sleep(self.poll_interval).await;
            }
        };
        fut.spawn_task();
        rx.into()
    }
}

/// A channel yeildiing responses from a poller task.
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
    Resp: RpcReturn + Clone,
{
    /// Resubscribe to the poller task.
    pub fn resubscribe(&self) -> Self {
        Self { rx: self.rx.resubscribe() }
    }

    /// Convert the poll channel into a stream.
    pub fn into_stream(self) -> BroadcastStream<Resp> {
        self.rx.into()
    }
}

#[cfg(test)]
#[allow(clippy::missing_const_for_fn)]
fn _assert_unpin() {
    fn _assert<T: Unpin>() {}
    _assert::<PollChannel<()>>();
}
