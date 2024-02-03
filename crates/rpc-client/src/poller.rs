use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};

use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_transport::{utils::Spawnable, Transport};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::WeakClient;

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
    limit: Option<usize>,

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
            limit: None,
            _pd: PhantomData,
        }
    }

    /// Set a limit on the number of succesful polls.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit;
    }

    /// Set the duration between polls.
    pub fn set_poll_interval(&mut self, poll_interval: Duration) {
        self.poll_interval = poll_interval;
    }

    /// Set the duration between polls.
    pub fn withpoll_interval(mut self, poll_interval: Duration) -> Self {
        self.set_poll_interval(poll_interval);
        self
    }

    /// Spawn the poller task, producing a stream of responses.
    pub fn spawn(self) -> PollChannel<Resp> {
        let (tx, rx) = broadcast::channel(self.channel_size);

        let fut = async move {
            let limit = self.limit.unwrap_or(usize::MAX);
            for _ in 0..limit {
                let client = match self.client.upgrade() {
                    Some(client) => client,
                    None => break,
                };

                match client.prepare(self.method, &self.params).await {
                    Ok(resp) => {
                        if tx.send(resp).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!(%e, "Error response in polling request.");
                    }
                }

                tokio::time::sleep(self.poll_interval).await;
            }
        };
        fut.spawn_task();
        rx.into()
    }
}

/// A stream of responses from a poller task.
///
/// This stream is backed by a coroutine, and will continue to produce responses
/// until the poller task is dropped. The poller task is dropped when all
/// [`RpcClient`] instances are dropped, or when all listening PollStream are
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

    /// Convert the poll_channel into a stream.
    pub fn into_stream(self) -> BroadcastStream<Resp> {
        BroadcastStream::from(self.rx)
    }
}
