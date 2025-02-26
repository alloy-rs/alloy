use crate::{provider::WithBlockInner, ParamsWithBlock, ProviderCall, WithBlock};
use alloy_eips::BlockId;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::map::HashSet;
use alloy_rpc_client::RpcCall;
use alloy_rpc_types_trace::parity::TraceType;
use alloy_transport::TransportResult;
use std::future::IntoFuture;

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
            trace_types: HashSet::default(),
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
            trace_types: HashSet::default(),
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

impl<Params, Resp, Output, Map> WithBlock for TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = block_id;
        self
    }
}

impl<Params, Resp, Output, Map> TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + 'static,
{
    /// Set the trace type.
    pub fn trace_type(mut self, trace_type: TraceType) -> Self {
        self.trace_types.insert(trace_type);
        self
    }

    /// Set the trace types.
    pub fn trace_types<I: IntoIterator<Item = TraceType>>(mut self, trace_types: I) -> Self {
        self.trace_types.extend(trace_types);
        self
    }

    /// Set the trace type to "trace".
    pub fn trace(self) -> Self {
        self.trace_type(TraceType::Trace)
    }

    /// Set the trace type to "vmTrace".
    pub fn vm_trace(self) -> Self {
        self.trace_type(TraceType::VmTrace)
    }

    /// Set the trace type to "stateDiff".
    pub fn state_diff(self) -> Self {
        self.trace_type(TraceType::StateDiff)
    }

    /// Get the trace types.
    pub const fn get_trace_types(&self) -> &HashSet<TraceType> {
        &self.trace_types
    }
}

impl<Params, Resp, Output, Map> IntoFuture for TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output + 'static,
{
    type Output = TransportResult<Output>;

    type IntoFuture = ProviderCall<ParamsWithBlock<Params>, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        match self.inner {
            WithBlockInner::RpcCall(inner) => {
                let block_id = self.block_id;
                let mut trace_types = self.trace_types;
                if trace_types.is_empty() {
                    trace_types.insert(TraceType::Trace);
                }
                let inner = inner.map_params(|params| {
                    ParamsWithBlock::new(params, block_id).with_trace_types(trace_types.clone())
                });
                ProviderCall::RpcCall(inner)
            }
            WithBlockInner::ProviderCall(get_call) => get_call(self.block_id),
        }
    }
}
