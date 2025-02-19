use crate::{ParamsWithBlock, ProviderCall, RpcWithBlock};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_rpc_client::{RpcCall, WeakClient};
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
    Map: Fn(Resp) -> Output + Clone,
{
    inner: RpcWithBlock<Params, Resp, Output, Map>,
    trace_types: HashSet<TraceType>,
}

impl<Params, Resp, Output, Map> Deref for TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    type Target = RpcWithBlock<Params, Resp, Output, Map>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<Params, Resp> TraceRpcWithBlock<Params, Resp>
where
    Params: RpcSend,
    Resp: RpcRecv,
{
    /// Create a new [`TraceRpcWithBlock`] instance.
    pub fn new(client: WeakClient, method: impl Into<Cow<'static, str>>, params: Params) -> Self {
        todo!()
        // Self {
        //     inner: RpcWithBlock::new_rpc(inner)
        //     trace_types: vec![TraceType::Trace].into_iter().collect(),
        // }
    }
}

impl<Params, Resp, Output, Map> TraceRpcWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone + 'static,
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
    Map: Fn(Resp) -> Output + 'static + Copy,
{
    type Output = TransportResult<Output>;
    type IntoFuture = ProviderCall<ParamsWithBlock<Params>, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        let inner: RpcWithBlock<Params, Resp, Output, Map> = self.into();
        inner.into_future()
    }
}
