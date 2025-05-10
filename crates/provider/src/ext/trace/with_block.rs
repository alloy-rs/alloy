use crate::ProviderCall;
use alloy_eips::BlockId;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::{map::HashSet, B256};
use alloy_rpc_client::RpcCall;
use alloy_rpc_types_trace::parity::TraceType;
use alloy_transport::TransportResult;
use std::future::IntoFuture;

/// A builder for trace_* api calls.
#[derive(Debug)]
pub struct TraceBuilder<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: WithBlockInner<Params, Resp, Output, Map>,
    block_id: Option<BlockId>,
    trace_types: Option<HashSet<TraceType>>,
}

impl<Params, Resp, Output, Map> TraceBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    /// Create a new [`TraceBuilder`] from a [`RpcCall`].
    pub const fn new_rpc(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self { inner: WithBlockInner::RpcCall(inner), block_id: None, trace_types: None }
    }

    /// Create a new [`TraceBuilder`] from a closure producing a [`ProviderCall`].
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
    for TraceBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn from(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self::new_rpc(inner)
    }
}

impl<F, Params, Resp, Output, Map> From<F> for TraceBuilder<Params, Resp, Output, Map>
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

impl<Params, Resp, Output, Map> TraceBuilder<Params, Resp, Output, Map>
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

    /// Set the block id.
    pub const fn block_id(mut self, block_id: BlockId) -> Self {
        self.block_id = Some(block_id);
        self
    }

    /// Set the block id to "pending".
    pub const fn pending(self) -> Self {
        self.block_id(BlockId::pending())
    }

    /// Set the block id to "latest".
    pub const fn latest(self) -> Self {
        self.block_id(BlockId::latest())
    }

    /// Set the block id to "earliest".
    pub const fn earliest(self) -> Self {
        self.block_id(BlockId::earliest())
    }

    /// Set the block id to "finalized".
    pub const fn finalized(self) -> Self {
        self.block_id(BlockId::finalized())
    }

    /// Set the block id to "safe".
    pub const fn safe(self) -> Self {
        self.block_id(BlockId::safe())
    }

    /// Set the block id to a specific height.
    pub const fn number(self, number: u64) -> Self {
        self.block_id(BlockId::number(number))
    }

    /// Set the block id to a specific hash, without requiring the hash be part
    /// of the canonical chain.
    pub const fn hash(self, hash: B256) -> Self {
        self.block_id(BlockId::hash(hash))
    }

    /// Set the block id to a specific hash and require the hash be part of the
    /// canonical chain.
    pub const fn hash_canonical(self, hash: B256) -> Self {
        self.block_id(BlockId::hash_canonical(hash))
    }
}

impl<Params, Resp, Output, Map> IntoFuture for TraceBuilder<Params, Resp, Output, Map>
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
                    TraceParams::new(&method, params, block_id, trace_types.clone())
                });
                ProviderCall::RpcCall(inner)
            }
            WithBlockInner::ProviderCall(get_call) => get_call(self.block_id),
        }
    }
}

/// Parameters for a trace call.
///
/// Contains optional block id and trace types to accommodate `trace_*` api calls that don't require
/// them.
#[derive(Debug, Clone)]
pub struct TraceParams<Params: RpcSend> {
    params: Params,
    block_id: Option<BlockId>,
    trace_types: Option<HashSet<TraceType>>,
}

impl<Params: RpcSend> TraceParams<Params> {}

impl<Params: RpcSend> serde::Serialize for TraceParams<Params> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        // Calculate tuple length based on optional fields
        let len = 1 + self.trace_types.is_some() as usize + self.block_id.is_some() as usize;

        let mut tup = serializer.serialize_tuple(len)?;

        // Always serialize params first
        tup.serialize_element(&self.params)?;

        // Add trace_types if present
        if let Some(trace_types) = &self.trace_types {
            tup.serialize_element(trace_types)?;
        }

        // Add block_id last if present
        if let Some(block_id) = &self.block_id {
            tup.serialize_element(block_id)?;
        }

        tup.end()
    }
}

impl<Params: RpcSend> TraceParams<Params> {
    /// Create a new `TraceParams` with the given parameters.
    ///
    /// The `method` is used to determine which parameters to ignore according to the `trace_*` api
    /// spec. See <https://reth.rs/jsonrpc/trace.html>.
    pub fn new(
        method: &String,
        params: Params,
        block_id: Option<BlockId>,
        trace_types: Option<HashSet<TraceType>>,
    ) -> Self {
        let block_id = block_id.unwrap_or(BlockId::pending());
        let trace_types = trace_types.unwrap_or_else(|| {
            let mut set = HashSet::default();
            set.insert(TraceType::Trace);
            set
        });
        match method.as_str() {
            "trace_call" => {
                Self { params, block_id: Some(block_id), trace_types: Some(trace_types) }
            }
            "trace_callMany" => {
                // Trace types are ignored as they are set per-tx-request in `params`.
                Self { params, block_id: Some(block_id), trace_types: None }
            }
            "trace_replayTransaction"
            | "trace_rawTransaction"
            | "trace_replayBlockTransactions" => {
                // BlockId is ignored
                Self { params, block_id: None, trace_types: Some(trace_types) }
            }
            _ => {
                unreachable!("{method} is not supported by TraceBuilder due to custom serialization requirements");
            }
        }
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
