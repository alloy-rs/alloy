use std::{fmt::Debug, marker::PhantomData};

use crate::{utils, ProviderCall};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcRecv;
use alloy_network::BlockResponse;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_primitives::{Address, BlockHash, B256, B64};
use alloy_rpc_client::{ClientRef, RpcCall};
#[cfg(feature = "pubsub")]
use alloy_rpc_types_eth::pubsub::SubscriptionKind;
use alloy_transport::{TransportError, TransportResult};
use either::Either;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::time::Duration;

use super::FilterPollerBuilder;

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
    pub const fn new(block: BlockId, kind: BlockTransactionsKind) -> Self {
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
    pub const fn kind(mut self, kind: BlockTransactionsKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the [`BlockTransactionsKind`] to [`BlockTransactionsKind::Full`].
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Set the [`BlockTransactionsKind`] to [`BlockTransactionsKind::Hashes`].
    pub const fn hashes(mut self) -> Self {
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
                    if block.get("hash").is_none_or(|v| v.is_null()) {
                        block["hash"] = Value::String(format!("{}", B256::ZERO));
                    }

                    if block.get("nonce").is_none_or(|v| v.is_null()) {
                        block["nonce"] = Value::String(format!("{}", B64::ZERO));
                    }

                    if block.get("miner").is_none_or(|v| v.is_null())
                        || block.get("beneficiary").is_none_or(|v| v.is_null())
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

/// A builder type for polling new blocks using the [`FilterPollerBuilder`].
///
/// By default, this polls for blocks with [`BlockTransactionsKind::Hashes`].
///
/// [`WatchBlocks::full`] should be used to poll for blocks with
/// [`BlockTransactionsKind::Full`].
///
/// The polling stream must be consumed by calling [`WatchBlocks::into_stream`].
#[derive(Debug)]
#[must_use = "this builder does nothing unless you call `.into_stream`"]
pub struct WatchBlocks<BlockResp> {
    /// [`PollerBuilder`] for polling new block hashes.
    ///
    /// On every poll it returns an array of block hashes [`Vec<B256>`] as `eth_getFilterChanges`
    /// returns an array of logs. See <https://docs.alchemy.com/reference/eth-getfilterchanges-1>.
    ///
    /// [`PollerBuilder`]: alloy_rpc_client::PollerBuilder
    poller: FilterPollerBuilder<B256>,
    /// The [`BlockTransactionsKind`] to retrieve on each poll.
    ///
    /// Default is [`BlockTransactionsKind::Hashes`].
    ///
    /// [`WatchBlocks::full`] should be used to poll for blocks with
    /// [`BlockTransactionsKind::Full`].
    kind: BlockTransactionsKind,
    _pd: std::marker::PhantomData<BlockResp>,
}

impl<BlockResp> WatchBlocks<BlockResp>
where
    BlockResp: BlockResponse + RpcRecv,
{
    /// Create a new [`WatchBlocks`] instance.
    pub(crate) const fn new(poller: FilterPollerBuilder<B256>) -> Self {
        Self { poller, kind: BlockTransactionsKind::Hashes, _pd: PhantomData }
    }

    /// Poll for blocks with full transactions i.e [`BlockTransactionsKind::Full`].
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Poll for blocks with just transactions hashes i.e [`BlockTransactionsKind::Hashes`].
    pub const fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
        self
    }

    /// Sets the channel size for the poller task.
    pub const fn set_channel_size(&mut self, channel_size: usize) {
        self.poller.set_channel_size(channel_size);
    }

    /// Sets a limit on the number of successful polls.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.poller.set_limit(limit);
    }

    /// Sets the duration between polls.
    pub const fn set_poll_interval(&mut self, poll_interval: Duration) {
        self.poller.set_poll_interval(poll_interval);
    }

    /// Consumes the stream of block hashes from the inner [`FilterPollerBuilder`] and maps it to a
    /// stream of [`BlockResponse`].
    pub fn into_stream(self) -> impl Stream<Item = TransportResult<BlockResp>> + Unpin {
        let client = self.poller.client();
        let kind = self.kind;
        let stream = self
            .poller
            .into_stream()
            .then(move |hashes| utils::hashes_to_blocks(hashes, client.clone(), kind.into()))
            .flat_map(|res| {
                futures::stream::iter(match res {
                    Ok(blocks) => {
                        // Ignore `None` responses.
                        Either::Left(blocks.into_iter().filter_map(|block| block.map(Ok)))
                    }
                    Err(err) => Either::Right(std::iter::once(Err(err))),
                })
            });
        Box::pin(stream)
    }
}

/// A builder type for subscribing to full blocks i.e [`alloy_network_primitives::BlockResponse`],
/// and not just [`alloy_network_primitives::HeaderResponse`].
///
/// By default this subscribes to block with tx hashes only. Use [`SubFullBlocks::full`] to
/// subscribe to blocks with full transactions.
#[derive(Debug)]
#[must_use = "this does nothing unless you call `.into_stream`"]
#[cfg(feature = "pubsub")]
pub struct SubFullBlocks<N: alloy_network::Network> {
    sub: super::GetSubscription<(SubscriptionKind,), N::HeaderResponse>,
    client: alloy_rpc_client::WeakClient,
    kind: BlockTransactionsKind,
}

#[cfg(feature = "pubsub")]
impl<N: alloy_network::Network> SubFullBlocks<N> {
    /// Create a new [`SubFullBlocks`] subscription with the given [`super::GetSubscription`].
    ///
    /// By default, this subscribes to block with tx hashes only. Use [`SubFullBlocks::full`] to
    /// subscribe to blocks with full transactions.
    pub const fn new(
        sub: super::GetSubscription<(SubscriptionKind,), N::HeaderResponse>,
        client: alloy_rpc_client::WeakClient,
    ) -> Self {
        Self { sub, client, kind: BlockTransactionsKind::Hashes }
    }

    /// Subscribe to blocks with full transactions.
    pub const fn full(mut self) -> Self {
        self.kind = BlockTransactionsKind::Full;
        self
    }

    /// Subscribe to blocks with transaction hashes only.
    pub const fn hashes(mut self) -> Self {
        self.kind = BlockTransactionsKind::Hashes;
        self
    }

    /// Set the channel size
    pub fn channel_size(mut self, size: usize) -> Self {
        self.sub = self.sub.channel_size(size);
        self
    }

    /// Subscribe to the inner stream of headers and map them to block responses.
    pub async fn into_stream(
        self,
    ) -> TransportResult<impl Stream<Item = TransportResult<N::BlockResponse>> + Unpin> {
        use alloy_network_primitives::HeaderResponse;
        use futures::StreamExt;

        let sub = self.sub.await?;

        let stream = sub
            .into_stream()
            .then(move |resp| {
                let hash = resp.hash();
                let kind = self.kind;
                let client_weak = self.client.clone();

                async move {
                    let client = client_weak
                        .upgrade()
                        .ok_or(TransportError::local_usage_str("Client dropped"))?;

                    let call = client.request("eth_getBlockByHash", (hash, kind.is_full()));
                    let resp = call.await?;

                    if kind.is_hashes() {
                        Ok(utils::convert_to_hashes(resp))
                    } else {
                        Ok(resp)
                    }
                }
            })
            .filter_map(|result| futures::future::ready(result.transpose()));

        #[cfg(not(target_family = "wasm"))]
        {
            Ok(stream.boxed())
        }

        #[cfg(target_family = "wasm")]
        {
            Ok(stream.boxed_local())
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
            ProviderBuilder::new().connect_http("https://binance.llamarpc.com".parse().unwrap());

        let res = provider.get_block_by_number(BlockNumberOrTag::Pending).full().await;
        if let Err(err) = &res {
            if err.to_string().contains("no response") {
                // response can be flaky
                eprintln!("skipping flaky response: {err:?}");
                return;
            }
        }
        let _block = res.unwrap();
    }
}
