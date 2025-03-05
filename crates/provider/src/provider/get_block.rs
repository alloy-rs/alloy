use std::marker::PhantomData;

use crate::{utils, ProviderCall};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_primitives::BlockHash;
use alloy_rpc_client::{ClientRef, RpcCall};
use alloy_transport::TransportResult;

/// The parameters for an `eth_getBlockBy{Hash, Number}` RPC request.
///
/// Default is "latest" block with transaction hashes.
#[derive(Clone, Debug, Default)]
pub struct EthGetBlockParams {
    block: BlockId,
    kind: BlockTransactionsKind,
}

impl serde::Serialize for EthGetBlockParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;

        let mut tuple = serializer.serialize_tuple(2)?;
        match self.block {
            BlockId::Hash(hash) => tuple.serialize_element(&hash.block_hash)?,
            BlockId::Number(number) => tuple.serialize_element(&number)?,
        }
        if self.kind.is_hashes() {
            tuple.serialize_element(&false)?;
        } else {
            tuple.serialize_element(&true)?
        };
        tuple.end()
    }
}

impl EthGetBlockParams {
    /// Instantiate [`EthGetBlockParams`] with the given block and kind.
    pub fn new(block: BlockId, kind: BlockTransactionsKind) -> Self {
        Self { block, kind }
    }
}

/// A builder for an `"eth_getBlockByHash"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthGetBlockBy must be awaited to execute the request"]
//#[derive(Clone, Debug)]
pub struct EthGetBlock<N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: GetBlockInner<Resp, Output, Map>,
    block: BlockId,
    kind: BlockTransactionsKind,
    _pd: std::marker::PhantomData<N>,
}

impl<N> EthGetBlock<N, Option<N::BlockResponse>>
where
    N: Network,
{
    /// Create a new [`EthGetBlock`] request to get the block by hash i.e call
    /// `"eth_getBlockByHash"`.
    pub fn by_hash(hash: BlockHash, client: ClientRef<'_>) -> Self {
        let params = EthGetBlockParams::default();
        let call = client.request("eth_getBlockByHash", params).map_resp(
            utils::convert_to_hashes::<N>
                as fn(Option<N::BlockResponse>) -> Option<N::BlockResponse>,
        );
        EthGetBlock::<N, Option<N::BlockResponse>>::new_rpc(hash.into(), call)
    }

    /// Create a new [`EthGetBlock`] request to get the block by number i.e call
    /// `"eth_getBlockByNumber"`.
    pub fn by_number(number: BlockNumberOrTag, client: ClientRef<'_>) -> Self {
        let params = EthGetBlockParams::default();
        let call = client.request("eth_getBlockByNumber", params).map_resp(
            utils::convert_to_hashes::<N>
                as fn(Option<N::BlockResponse>) -> Option<N::BlockResponse>,
        );
        EthGetBlock::<N, Option<N::BlockResponse>>::new_rpc(number.into(), call)
    }
}

impl<N, Resp, Output, Map> EthGetBlock<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Create a new [`EthGetBlock`] request with the given [`RpcCall`].
    pub fn new_rpc(block: BlockId, inner: RpcCall<EthGetBlockParams, Resp, Output, Map>) -> Self {
        Self {
            block,
            inner: GetBlockInner::RpcCall(inner),
            kind: BlockTransactionsKind::Hashes,
            _pd: PhantomData,
        }
    }

    /// Create a new [`EthGetBlock`] request with a closure that returns a [`ProviderCall`].
    pub fn new_provider(block: BlockId, producer: ProviderCallProducer<Resp, Output, Map>) -> Self {
        Self {
            block,
            inner: GetBlockInner::ProviderCall(producer),
            kind: BlockTransactionsKind::Hashes,
            _pd: PhantomData,
        }
    }

    /// Set the [`BlockTransactionsKind`] for the request.
    pub fn kind(mut self, kind: BlockTransactionsKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the [`BlockTransactionsKind`] to [`BlockTransactionsKind::Full`].
    pub fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Set the [`BlockTransactionsKind`] to [`BlockTransactionsKind::Hashes`].
    pub fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
        self
    }
}

impl<N, Resp, Output, Map> std::future::IntoFuture for EthGetBlock<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = ProviderCall<EthGetBlockParams, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        match self.inner {
            GetBlockInner::RpcCall(call) => {
                let rpc_call =
                    call.map_params(|_params| EthGetBlockParams::new(self.block, self.kind));
                ProviderCall::RpcCall(rpc_call)
            }
            GetBlockInner::ProviderCall(producer) => producer(self.kind),
        }
    }
}

impl<N, Resp, Output, Map> core::fmt::Debug for EthGetBlock<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthGetBlock").field("kind", &self.kind).finish()
    }
}

type ProviderCallProducer<Resp, Output, Map> =
    Box<dyn Fn(BlockTransactionsKind) -> ProviderCall<EthGetBlockParams, Resp, Output, Map> + Send>;
enum GetBlockInner<Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// [`RpcCall`] with params that get wrapped into [`EthGetBlockParams`] in the future.
    RpcCall(RpcCall<EthGetBlockParams, Resp, Output, Map>),
    /// Closure that produces a [`ProviderCall`] given [`BlockTransactionsKind`].
    ProviderCall(ProviderCallProducer<Resp, Output, Map>),
}

impl<Resp, Output, Map> core::fmt::Debug for GetBlockInner<Resp, Output, Map>
where
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
