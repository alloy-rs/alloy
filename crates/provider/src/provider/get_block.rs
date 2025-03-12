use std::{fmt::Debug, marker::PhantomData};

use crate::{utils, ProviderCall};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcRecv;
use alloy_network::BlockResponse;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_primitives::{Address, BlockHash, B256, B64};
use alloy_rpc_client::{ClientRef, RpcCall};
use alloy_transport::{TransportError, TransportResult};
use serde_json::Value;

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
pub struct EthGetBlock<BlockResp>
where
    BlockResp: alloy_network::BlockResponse + RpcRecv,
{
    inner: GetBlockInner<BlockResp>,
    block: BlockId,
    kind: BlockTransactionsKind,
    _pd: std::marker::PhantomData<BlockResp>,
}

impl<BlockResp> EthGetBlock<BlockResp>
where
    BlockResp: alloy_network::BlockResponse + RpcRecv,
{
    /// Create a new [`EthGetBlock`] request to get the block by hash i.e call
    /// `"eth_getBlockByHash"`.
    pub fn by_hash(hash: BlockHash, client: ClientRef<'_>) -> Self {
        let params = EthGetBlockParams::default();
        let call = client.request("eth_getBlockByHash", params);
        Self::new_rpc(hash.into(), call)
    }

    /// Create a new [`EthGetBlock`] request to get the block by number i.e call
    /// `"eth_getBlockByNumber"`.
    pub fn by_number(number: BlockNumberOrTag, client: ClientRef<'_>) -> Self {
        let params = EthGetBlockParams::default();

        if number.is_pending() {
            return Self::new_pending_rpc(client.request("eth_getBlockByNumber", params));
        }

        Self::new_rpc(number.into(), client.request("eth_getBlockByNumber", params))
    }
}

impl<BlockResp> EthGetBlock<BlockResp>
where
    BlockResp: alloy_network::BlockResponse + RpcRecv,
{
    /// Create a new [`EthGetBlock`] request with the given [`RpcCall`].
    pub fn new_rpc(block: BlockId, inner: RpcCall<EthGetBlockParams, Option<BlockResp>>) -> Self {
        Self {
            block,
            inner: GetBlockInner::RpcCall(inner),
            kind: BlockTransactionsKind::Hashes,
            _pd: PhantomData,
        }
    }

    /// Create a new [`EthGetBlock`] request with the given [`RpcCall`] for pending block.
    pub fn new_pending_rpc(inner: RpcCall<EthGetBlockParams, Value>) -> Self {
        Self {
            block: BlockId::pending(),
            inner: GetBlockInner::PendingBlock(inner),
            kind: BlockTransactionsKind::Hashes,
            _pd: PhantomData,
        }
    }

    /// Create a new [`EthGetBlock`] request with a closure that returns a [`ProviderCall`].
    pub fn new_provider(block: BlockId, producer: ProviderCallProducer<BlockResp>) -> Self {
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

impl<BlockResp> std::future::IntoFuture for EthGetBlock<BlockResp>
where
    BlockResp: alloy_network::BlockResponse + RpcRecv,
{
    type Output = TransportResult<Option<BlockResp>>;

    type IntoFuture = ProviderCall<EthGetBlockParams, Option<BlockResp>>;

    fn into_future(self) -> Self::IntoFuture {
        match self.inner {
            GetBlockInner::RpcCall(call) => {
                let rpc_call =
                    call.map_params(|_params| EthGetBlockParams::new(self.block, self.kind));

                let fut = async move {
                    let resp = rpc_call.await?;
                    let result =
                        if self.kind.is_hashes() { utils::convert_to_hashes(resp) } else { resp };
                    Ok(result)
                };

                ProviderCall::BoxedFuture(Box::pin(fut))
            }
            GetBlockInner::PendingBlock(call) => {
                let rpc_call =
                    call.map_params(|_params| EthGetBlockParams::new(self.block, self.kind));

                let map_fut = async move {
                    let mut block = rpc_call.await?;

                    if block.is_null() {
                        return Ok(None);
                    }

                    // Ref: <https://github.com/alloy-rs/alloy/issues/2117>
                    // Geth ref: <https://github.com/ethereum/go-ethereum/blob/ebff2f42c0fbb4ebee43b0e73e39b658305a8a9b/internal/ethapi/api.go#L470-L471>
                    tracing::trace!(pending_block = ?block.to_string());
                    if block.get("hash").map_or(true, |v| v.is_null()) {
                        block["hash"] = Value::String(format!("{}", B256::ZERO));
                    }

                    if block.get("nonce").map_or(true, |v| v.is_null()) {
                        block["nonce"] = Value::String(format!("{}", B64::ZERO));
                    }

                    if block.get("miner").map_or(true, |v| v.is_null())
                        || block.get("beneficiary").map_or(true, |v| v.is_null())
                    {
                        block["miner"] = Value::String(format!("{}", Address::ZERO));
                    }

                    let block = serde_json::from_value(block.clone())
                        .map_err(|e| TransportError::deser_err(e, block.to_string()))?;

                    let block = if self.kind.is_hashes() {
                        utils::convert_to_hashes(Some(block))
                    } else {
                        Some(block)
                    };

                    Ok(block)
                };

                ProviderCall::BoxedFuture(Box::pin(map_fut))
            }
            GetBlockInner::ProviderCall(producer) => producer(self.kind),
        }
    }
}

impl<BlockResp> core::fmt::Debug for EthGetBlock<BlockResp>
where
    BlockResp: BlockResponse + RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthGetBlock").field("block", &self.block).field("kind", &self.kind).finish()
    }
}

type ProviderCallProducer<BlockResp> =
    Box<dyn Fn(BlockTransactionsKind) -> ProviderCall<EthGetBlockParams, Option<BlockResp>> + Send>;

enum GetBlockInner<BlockResp>
where
    BlockResp: BlockResponse + RpcRecv,
{
    /// [`RpcCall`] with params that get wrapped into [`EthGetBlockParams`] in the future.
    RpcCall(RpcCall<EthGetBlockParams, Option<BlockResp>>),
    /// Pending Block Call
    ///
    /// This has been made explicit to handle cases where fields such as `hash`, `nonce`, `miner`
    /// are either missing or set to null causing deserilization issues. See: <https://github.com/alloy-rs/alloy/issues/2117>
    ///
    /// This is specifically true in case of the response is returned from a geth node. See: <https://github.com/ethereum/go-ethereum/blob/ebff2f42c0fbb4ebee43b0e73e39b658305a8a9b/internal/ethapi/api.go#L470-L471>
    ///
    /// In such case, we first deserialize to [`Value`] and then check if the fields are missing or
    /// set to null. If so, we set them to default values.  
    PendingBlock(RpcCall<EthGetBlockParams, Value>),
    /// Closure that produces a [`ProviderCall`] given [`BlockTransactionsKind`].
    ProviderCall(ProviderCallProducer<BlockResp>),
}

impl<BlockResp> core::fmt::Debug for GetBlockInner<BlockResp>
where
    BlockResp: BlockResponse + RpcRecv,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RpcCall(call) => f.debug_tuple("RpcCall").field(call).finish(),
            Self::PendingBlock(call) => f.debug_tuple("PendingBlockCall").field(call).finish(),
            Self::ProviderCall(_) => f.debug_struct("ProviderCall").finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Provider, ProviderBuilder};

    // <https://github.com/alloy-rs/alloy/issues/2117>
    #[tokio::test]
    async fn test_pending_block_deser() {
        let provider =
            ProviderBuilder::new().on_http("https://binance.llamarpc.com".parse().unwrap());

        let _block = provider.get_block_by_number(BlockNumberOrTag::Pending).await.unwrap();
    }
}
