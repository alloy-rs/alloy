use crate::{RpcWithBlock, RpcWithBlockFut};
use alloy_json_rpc::{RpcParam, RpcReturn};
use alloy_rpc_client::WeakClient;
use alloy_rpc_types_trace::parity::TraceType;
use alloy_transport::{Transport, TransportResult};
use std::{borrow::Cow, collections::HashSet, future::IntoFuture, ops::Deref};

/// An wrapper for [`TraceRpcWithBlock`] that takes an optional [`TraceType`] parameter. By default
/// this will use "trace".
#[derive(Debug, Clone)]
pub struct TraceRpcWithBlock<T, Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    inner: RpcWithBlock<T, Params, Resp, Output, Map>,
    trace_types: HashSet<TraceType>,
}

impl<T, Params, Resp, Output, Map> Deref for TraceRpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Map: Fn(Resp) -> Output,
{
    type Target = RpcWithBlock<T, Params, Resp, Output, Map>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, Params, Resp> TraceRpcWithBlock<T, Params, Resp>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    /// Create a new [`TraceRpcWithBlock`] instance.
    pub fn new(
        client: WeakClient<T>,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Self {
        Self {
            inner: RpcWithBlock::new(client, method, params),
            trace_types: vec![TraceType::Trace].into_iter().collect(),
        }
    }
}

impl<T, Params, Resp, Output, Map> TraceRpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
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

impl<T, Params, Resp, Output, Map> IntoFuture for TraceRpcWithBlock<T, Params, Resp, Output, Map>
where
    T: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
    Output: 'static,
    Map: Fn(Resp) -> Output + 'static + Copy,
{
    type Output = TransportResult<Output>;
    type IntoFuture = RpcWithBlockFut<T, Params, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        let inner: RpcWithBlock<T, Params, Resp, Output, Map> = self.into();
        inner.into_future()
    }
}
