use alloy_eips::BlockId;
use alloy_json_rpc::{RpcError, RpcParam, RpcReturn};
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use std::{borrow::Cow, future::IntoFuture, marker::PhantomData, task::Poll};

/// States of the
#[derive(Debug, Clone)]
enum States<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    Preparing {
        client: WeakClient<T>,
        method: Cow<'static, str>,
        params: Params,
        block_id: BlockId,
    },
    Running(RpcCall<T, serde_json::Value, Resp>),
}

/// A future for [`BlockIdRpc`]. Simple wrapper around [`RpcCall`].
#[derive(Debug, Clone)]
pub struct RpcWithBlockFut<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    state: States<T, Params, Resp>,
}

impl<T, Params, Resp> RpcWithBlockFut<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn poll_preparing(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Resp>> {
        let fut = {
            let States::Preparing { method, client, params, block_id } = &self.as_ref().state
            else {
                unreachable!("bad state")
            };

            // serialize the params, appending the block id
            let ser = serde_json::to_value(params).map_err(RpcError::ser_err);
            let mut ser = match ser {
                Ok(ser) => ser,
                Err(e) => return Poll::Ready(Err(e)),
            };
            let block_id = serde_json::to_value(block_id).map_err(RpcError::ser_err);
            let block_id = match block_id {
                Ok(block_id) => block_id,
                Err(e) => return Poll::Ready(Err(e)),
            };

            if let serde_json::Value::Array(ref mut arr) = ser {
                arr.push(block_id);
            } else if let serde_json::Value::Null = ser {
                ser = serde_json::Value::Array(vec![block_id]);
            } else {
                ser = serde_json::Value::Array(vec![ser, block_id]);
            }

            let client = match client.upgrade().ok_or_else(TransportErrorKind::backend_gone) {
                Ok(client) => client,
                Err(e) => return Poll::Ready(Err(e)),
            };

            client.request(method.clone(), ser)
        };
        self.state = States::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Resp>> {
        let Self { state: States::Running(call) } = self.get_mut() else {
            unreachable!("bad state")
        };

        call.poll_unpin(cx)
    }
}

impl<T, Params, Resp> std::future::Future for RpcWithBlockFut<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    type Output = TransportResult<Resp>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if matches!(self.state, States::Preparing { .. }) {
            self.poll_preparing(cx)
        } else {
            self.poll_running(cx)
        }
    }
}

/// An [`RpcCall`] that takes an optional [`BlockId`] parameter. By default
/// this will use "latest".
pub struct RpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    client: WeakClient<T>,
    method: Cow<'static, str>,
    params: Params,
    block_id: BlockId,
    _pd: PhantomData<Resp>,
}

impl<T, Params, Resp> RpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    /// Create a new [`RpcWithBlock`] instance.
    pub fn new(
        client: WeakClient<T>,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Self {
        Self {
            client,
            method: method.into(),
            params,
            block_id: Default::default(),
            _pd: PhantomData,
        }
    }

    /// Set the block id.
    pub fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = block_id;
        self
    }
}

impl<T, Params, Resp> IntoFuture for RpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    type Output = TransportResult<Resp>;

    type IntoFuture = RpcWithBlockFut<T, Params, Resp>;

    fn into_future(self) -> Self::IntoFuture {
        RpcWithBlockFut {
            state: States::Preparing {
                client: self.client,
                method: self.method,
                params: self.params,
                block_id: self.block_id,
            },
        }
    }
}
