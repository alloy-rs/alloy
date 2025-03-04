use crate::ProviderCall;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network_primitives::BlockTransactionsKind;
use alloy_rpc_client::RpcCall;
use alloy_transport::TransportResult;

/// The parameters for an `eth_getBlockBy{Hash, Number}` RPC request.
///
/// Default is "latest" block with transaction hashes.
#[derive(Clone, Debug)]
pub struct EthGetBlockParams<Params: RpcSend> {
    params: Params,
    kind: BlockTransactionsKind,
}

impl<Params: RpcSend> serde::Serialize for EthGetBlockParams<Params> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;

        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.params)?;
        if self.kind.is_hashes() {
            tuple.serialize_element(&false)?;
        } else {
            tuple.serialize_element(&true)?
        };
        tuple.end()
    }
}

impl<Params: RpcSend> EthGetBlockParams<Params> {
    fn new(params: Params, kind: BlockTransactionsKind) -> Self {
        Self { params, kind }
    }
}

/// A builder for an `"eth_getBlockByHash"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthGetBlockBy must be awaited to execute the request"]
//#[derive(Clone, Debug)]
pub struct EthGetBlock<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: GetBlockInner<Params, Resp, Output, Map>,
    kind: BlockTransactionsKind,
}

impl<Params, Resp, Output, Map> EthGetBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Create a new [`EthGetBlock`] request with the given [`RpcCall`].
    pub fn new_rpc(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self { inner: GetBlockInner::RpcCall(inner), kind: BlockTransactionsKind::Hashes }
    }

    /// Create a new [`EthGetBlock`] request with the given [`ProviderCallProducer`].
    pub fn new_provider(producer: ProviderCallProducer<Params, Resp, Output, Map>) -> Self {
        Self { inner: GetBlockInner::ProviderCall(producer), kind: BlockTransactionsKind::Hashes }
    }

    /// Set the [`BlockTransactionsKind`] for the request.
    pub fn kind(mut self, kind: BlockTransactionsKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the `full:bool` argument in RPC calls
    pub fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }
}

impl<Params, Resp, Output, Map> std::future::IntoFuture for EthGetBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = ProviderCall<EthGetBlockParams<Params>, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        match self.inner {
            GetBlockInner::RpcCall(call) => {
                let rpc_call = call.map_params(|params| EthGetBlockParams::new(params, self.kind));
                ProviderCall::RpcCall(rpc_call)
            }
            GetBlockInner::ProviderCall(producer) => producer(self.kind),
        }
    }
}

impl<Params, Resp, Output, Map> core::fmt::Debug for EthGetBlock<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthGetBlock").field("kind", &self.kind).finish()
    }
}

type ProviderCallProducer<Params, Resp, Output, Map> = Box<
    dyn Fn(BlockTransactionsKind) -> ProviderCall<EthGetBlockParams<Params>, Resp, Output, Map>
        + Send,
>;
enum GetBlockInner<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// [`RpcCall`] with params that get wrapped into [`EthGetBlockParams`] in the future.
    RpcCall(RpcCall<Params, Resp, Output, Map>),
    /// Closure that produces a [`ProviderCall`] given a [`BlockId`] and [`BlockTransactionsKind`].
    ProviderCall(ProviderCallProducer<Params, Resp, Output, Map>),
}

impl<Params, Resp, Output, Map> core::fmt::Debug for GetBlockInner<Params, Resp, Output, Map>
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
