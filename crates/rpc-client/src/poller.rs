use std::marker::PhantomData;

use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_transport::{utils::Spawnable, Transport, TransportResult};
use reqwest::Request;
use tokio::sync::broadcast;

use crate::WeakClient;

pub struct PollTask<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    client: WeakClient<Conn>,
    params: Params,
    method: &'static str,
    tx: broadcast::Sender<TransportResult<Resp>>,

    duration: std::time::Duration,
}

impl<Conn, Params, Resp> PollTask<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam + 'static,
    Resp: RpcReturn,
{
    /// Spawn the poller task.
    pub fn spawn(self) {
        let fut = async move {
            loop {
                tokio::time::sleep(self.duration).await;

                let client = match self.client.upgrade() {
                    Some(client) => client,
                    None => break,
                };
                let resp = client.prepare(self.method, &self.params).await;
                if self.tx.send(resp).is_err() {
                    break;
                }
            }
        };
        fut.spawn_task()
    }
}

#[derive(Debug)]
pub struct PollStream<Resp> {
    _pd: PhantomData<fn() -> Resp>,
    rx: tokio_stream::wrappers::BroadcastStream<Resp>,
}
