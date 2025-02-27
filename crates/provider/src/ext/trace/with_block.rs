use crate::{ProviderCall, WithBlock};
use alloy_eips::BlockId;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::map::HashSet;
use alloy_rpc_client::RpcCall;
use alloy_rpc_types_trace::parity::TraceType;
use alloy_transport::TransportResult;
use std::future::IntoFuture;

/// An wrapper for [`TraceWithBlock`] that takes an optional [`TraceType`] parameter. By default
/// this will use "trace".
#[derive(Debug)]
pub struct TraceWithBlock<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: WithBlockInner<Params, Resp, Output, Map>,
    block_id: Option<BlockId>,
    trace_types: Option<HashSet<TraceType>>,
}

impl<Params, Resp, Output, Map> TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    /// Create a new [`RpcWithBlock`] from a [`RpcCall`].
    pub fn new_rpc(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self { inner: WithBlockInner::RpcCall(inner), block_id: None, trace_types: None }
    }

    /// Create a new [`RpcWithBlock`] from a closure producing a [`ProviderCall`].
    pub fn new_provider<F>(get_call: F) -> Self
    where
        F: Fn(Option<BlockId>) -> ProviderCall<TraceParams<Params>, Resp, Output, Map>
            + Send
            + 'static,
    {
        let get_call = Box::new(get_call);

        Self { inner: WithBlockInner::ProviderCall(get_call), block_id: None, trace_types: None }
    }
}

impl<Params, Resp, Output, Map> From<RpcCall<Params, Resp, Output, Map>>
    for TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn from(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self::new_rpc(inner)
    }
}

impl<F, Params, Resp, Output, Map> From<F> for TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
    F: Fn(Option<BlockId>) -> ProviderCall<TraceParams<Params>, Resp, Output, Map> + Send + 'static,
{
    fn from(inner: F) -> Self {
        Self::new_provider(inner)
    }
}

impl<Params, Resp, Output, Map> WithBlock for TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = Some(block_id);
        self
    }
}

impl<Params, Resp, Output, Map> TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + 'static,
{
    /// Set the trace type.
    pub fn trace_type(mut self, trace_type: TraceType) -> Self {
        self.trace_types.get_or_insert_with(HashSet::default).insert(trace_type);
        self
    }

    /// Set the trace types.
    pub fn trace_types<I: IntoIterator<Item = TraceType>>(mut self, trace_types: I) -> Self {
        self.trace_types.get_or_insert_with(HashSet::default).extend(trace_types);
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
    pub const fn get_trace_types(&self) -> Option<&HashSet<TraceType>> {
        self.trace_types.as_ref()
    }

    pub fn get_params(&self, method: &str, params: Params) -> TraceParams<Params> {
        match method {
            "trace_call" => {
                let block_id = self.block_id.unwrap_or(BlockId::pending());
                let trace_types = self.trace_types.clone().unwrap_or_else(|| {
                    let mut set = HashSet::default();
                    set.insert(TraceType::Trace);
                    set
                });
                return TraceParams {
                    params,
                    block_id: Some(block_id),
                    trace_types: Some(trace_types),
                };
            }
            _ => {
                todo!()
            }
        }
    }
}

impl<Params, Resp, Output, Map> IntoFuture for TraceWithBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output + 'static,
{
    type Output = TransportResult<Output>;

    type IntoFuture = ProviderCall<TraceParams<Params>, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        match self.inner {
            WithBlockInner::RpcCall(inner) => {
                let block_id = self.block_id;
                let trace_types = self.trace_types;
                let method = inner.method().to_string();
                let inner = inner.map_params(|params| {
                    trace_params(&method, params, block_id, trace_types.clone())
                });
                ProviderCall::RpcCall(inner)
            }
            WithBlockInner::ProviderCall(get_call) => get_call(self.block_id),
        }
    }
}

fn trace_params<Params: RpcSend>(
    method: &String,
    params: Params,
    block_id: Option<BlockId>,
    trace_types: Option<HashSet<TraceType>>,
) -> TraceParams<Params> {
    match method.as_str() {
        "trace_call" => {
            let block_id = block_id.unwrap_or(BlockId::pending());
            let trace_types = trace_types.unwrap_or_else(|| {
                let mut set = HashSet::default();
                set.insert(TraceType::Trace);
                set
            });
            return TraceParams {
                params,
                block_id: Some(block_id),
                trace_types: Some(trace_types),
            };
        }
        "trace_callMany" => {
            let block_id = block_id.unwrap_or(BlockId::pending());
            // Trace types is ignored as it is set per request in `params`.
            return TraceParams { params, block_id: Some(block_id), trace_types: None };
        }
        _ => {
            todo!()
        }
    }
}

/// Parameters for a trace call.
///
/// Contains optional block id and trace types to accomodate `trace_*` api calls that don't require
/// them.
#[derive(Debug, Clone)]
pub struct TraceParams<Params: RpcSend> {
    params: Params,
    block_id: Option<BlockId>,
    trace_types: Option<HashSet<TraceType>>,
}

impl<Params: RpcSend> serde::Serialize for TraceParams<Params> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize params to a Value first
        let mut ser = serde_json::to_value(&self.params).map_err(serde::ser::Error::custom)?;

        // Convert to array if needed
        if !matches!(ser, serde_json::Value::Array(_)) {
            if ser.is_null() {
                ser = serde_json::Value::Array(Vec::new());
            } else {
                ser = serde_json::Value::Array(vec![ser]);
            }
        }

        // Get mutable reference to array
        let arr = ser.as_array_mut().unwrap();

        // Add trace_types if present
        if let Some(trace_types) = &self.trace_types {
            let trace_types =
                serde_json::to_value(trace_types).map_err(serde::ser::Error::custom)?;
            arr.push(trace_types);
        }

        // Add block_id last
        let block_id = serde_json::to_value(self.block_id).map_err(serde::ser::Error::custom)?;
        arr.push(block_id);

        ser.serialize(serializer)
    }
}

impl<Params: RpcSend> TraceParams<Params> {
    pub fn new(params: Params) -> Self {
        Self { params, block_id: None, trace_types: None }
    }

    pub fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = Some(block_id);
        self
    }

    pub fn trace_types(mut self, trace_types: HashSet<TraceType>) -> Self {
        self.trace_types = Some(trace_types);
        self
    }
}

/// Provider producers that create a [`ProviderCall`] with [`TraceParams`].
type ProviderCallProducer<Params, Resp, Output, Map> =
    Box<dyn Fn(Option<BlockId>) -> ProviderCall<TraceParams<Params>, Resp, Output, Map> + Send>;

enum WithBlockInner<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    RpcCall(RpcCall<Params, Resp, Output, Map>),
    ProviderCall(ProviderCallProducer<Params, Resp, Output, Map>),
}

impl<Params, Resp, Output, Map> core::fmt::Debug for WithBlockInner<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RpcCall(call) => f.debug_tuple("RpcCall").field(call).finish(),
            Self::ProviderCall(_) => f.debug_struct("ProviderCall").finish(),
        }
    }
}
