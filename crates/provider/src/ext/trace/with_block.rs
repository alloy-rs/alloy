use crate::{provider::WithBlockInner, ParamsWithBlock, ProviderCall};
use alloy_eips::BlockId;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_rpc_client::RpcCall;
use alloy_rpc_types_trace::parity::TraceType;
use alloy_transport::{Transport, TransportResult};
use std::{borrow::Cow, collections::HashSet, future::IntoFuture, ops::Deref};

/// An wrapper for [`TraceRpcWithBlock`] that takes an optional [`TraceType`] parameter. By default
/// this will use "trace".
#[derive(Debug)]
pub struct TraceRpcWithBlock<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: WithBlockInner<Params, Resp, Output, Map>,
    block_id: BlockId,
    trace_types: HashSet<TraceType>,
}

impl<Params, Resp, Output, Map> TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    /// Create a new [`RpcWithBlock`] from a [`RpcCall`].
    pub fn new_rpc(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self {
            inner: WithBlockInner::RpcCall(inner),
            block_id: Default::default(),
            trace_types: [TraceType::Trace].into(),
        }
    }

    /// Create a new [`RpcWithBlock`] from a closure producing a [`ProviderCall`].
    pub fn new_provider<F>(get_call: F) -> Self
    where
        F: Fn(BlockId) -> ProviderCall<ParamsWithBlock<Params>, Resp, Output, Map> + Send + 'static,
    {
        let get_call = Box::new(get_call);
        Self {
            inner: WithBlockInner::ProviderCall(get_call),
            block_id: Default::default(),
            trace_types: [TraceType::Trace].into(),
        }
    }
}

impl<Params, Resp, Output, Map> From<RpcCall<Params, Resp, Output, Map>>
    for TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn from(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self::new_rpc(inner)
    }
}

impl<F, Params, Resp, Output, Map> From<F> for TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
    F: Fn(BlockId) -> ProviderCall<ParamsWithBlock<Params>, Resp, Output, Map> + Send + 'static,
{
    fn from(inner: F) -> Self {
        Self::new_provider(inner)
    }
}
