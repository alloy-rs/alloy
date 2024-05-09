use alloy_eips::BlockId;
use alloy_json_rpc::{RpcError, RpcParam, RpcReturn};
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use futures::FutureExt;
use std::{
    borrow::Cow,
    future::{Future, IntoFuture},
    marker::PhantomData,
    task::Poll,
};

/// States of the
#[derive(Clone)]
enum States<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    Preparing {
        client: WeakClient<T>,
        method: Cow<'static, str>,
        params: Params,
        block_id: BlockId,
        map: Map,
    },
    Running(RpcCall<T, serde_json::Value, Resp, Output, Map>),
}

impl<T, Params, Resp, Output, Map> core::fmt::Debug for States<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preparing { client, method, params, block_id, .. } => f
                .debug_struct("Preparing")
                .field("client", client)
                .field("method", method)
                .field("params", params)
                .field("block_id", block_id)
                .finish(),
            Self::Running(arg0) => f.debug_tuple("Running").field(arg0).finish(),
        }
    }
}

/// A future for [`RpcWithBlock`]. Simple wrapper around [`RpcCall`].
#[derive(Debug, Clone)]
pub struct RpcWithBlockFut<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    state: States<T, Params, Resp, Output, Map>,
}

impl<T, Params, Resp, Output, Map> RpcWithBlockFut<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone + Unpin,
{
    fn poll_preparing(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let fut = {
            let States::Preparing { method, client, params, block_id, map } = &self.as_ref().state
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

            client.request(method.clone(), ser).map_resp(map.clone())
        };
        self.state = States::Running(fut);
        self.poll_running(cx)
    }

    fn poll_running(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<TransportResult<Output>> {
        let Self { state: States::Running(call) } = self.get_mut() else {
            unreachable!("bad state")
        };

        call.poll_unpin(cx)
    }
}

impl<T, Params, Resp, Output, Map> Future for RpcWithBlockFut<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone + Unpin,
{
    type Output = TransportResult<Output>;

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
#[derive(Debug, Clone)]
pub struct RpcWithBlock<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    client: WeakClient<T>,
    method: Cow<'static, str>,
    params: Params,
    block_id: BlockId,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
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
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<T, Params, Resp, Output, Map> RpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    /// Map the response.
    pub fn map_resp<NewOutput, NewMap>(
        self,
        map: NewMap,
    ) -> RpcWithBlock<T, Params, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        RpcWithBlock {
            client: self.client,
            method: self.method,
            params: self.params,
            block_id: self.block_id,
            map,
            _pd: PhantomData,
        }
    }

    /// Set the block id.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = block_id;
        self
    }
}

impl<T, Params, Resp, Output, Map> IntoFuture for RpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + Clone + Unpin,
{
    type Output = TransportResult<Output>;

    type IntoFuture = RpcWithBlockFut<T, Params, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        RpcWithBlockFut {
            state: States::Preparing {
                client: self.client,
                method: self.method,
                params: self.params,
                block_id: self.block_id,
                map: self.map,
            },
        }
    }
}
