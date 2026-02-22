use super::FilterPollerBuilder;
use crate::{utils, ProviderCall};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcRecv;
use alloy_network::BlockResponse;
use alloy_network_primitives::BlockTransactionsKind;
use alloy_primitives::{Address, BlockHash, B256, B64};
use alloy_rpc_client::{ClientRef, RpcCall};
use alloy_transport::{TransportError, TransportResult};
use either::Either;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::{fmt::Debug, marker::PhantomData, time::Duration};
#[cfg(feature = "pubsub")]
use {
    alloy_consensus::BlockHeader, alloy_rpc_types_eth::pubsub::SubscriptionKind,
    alloy_transport::BoxFuture, futures::ready, futures::task::Poll, futures::FutureExt,
    std::future::IntoFuture, std::pin::Pin,
};

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
    /// are either missing or set to null causing deserialization issues. See: <https://github.com/alloy-rs/alloy/issues/2117>
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

/// A builder type for subscribing to finalized blocks with both latest and finalized block info.
///
/// By default this subscribes to blocks with tx hashes only. Use [`SubFinalizedBlocks::full`] to
/// subscribe to blocks with full transactions.
#[derive(Debug)]
#[must_use = "this does nothing unless you call `.into_stream`"]
#[cfg(feature = "pubsub")]
pub struct SubFinalizedBlocks<N: alloy_network::Network> {
    sub: super::GetSubscription<(SubscriptionKind,), N::HeaderResponse>,
    client: alloy_rpc_client::WeakClient,
    kind: BlockTransactionsKind,
}

#[cfg(feature = "pubsub")]
impl<N: alloy_network::Network> SubFinalizedBlocks<N> {
    /// Create a new [`SubFinalizedBlocks`] subscription with the given [`super::GetSubscription`].
    ///
    /// By default, this subscribes to blocks with tx hashes only. Use [`SubFinalizedBlocks::full`]
    /// to subscribe to blocks with full transactions.
    ///
    /// The stream yields tuples of `(HeaderResponse, Option<BlockResponse>)` where the first
    /// element is the latest block header and the second is the finalized block.
    pub fn new(
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

    /// Creates a stream that yields tuples of `(HeaderResponse, Option<BlockResponse>)`
    /// containing both the latest block header and the latest finalized block.
    pub async fn into_stream(self) -> TransportResult<FinalizedBlocksStream<N>> {
        let sub = self.sub.await?;
        Ok(FinalizedBlocksStream::new(sub, self.client, self.kind))
    }
}

/// A stream of finalized blocks that yields both the latest block header and finalized block.
#[cfg(feature = "pubsub")]
pub struct FinalizedBlocksStream<N: alloy_network::Network> {
    inner: Pin<Box<dyn Stream<Item = N::HeaderResponse> + Send>>,
    client: alloy_rpc_client::WeakClient,
    kind: BlockTransactionsKind,
    pending_request: Option<BoxFuture<'static, TransportResult<Option<N::BlockResponse>>>>,
    current_header: Option<N::HeaderResponse>,
    /// Cached finalized block to avoid unnecessary requests
    cached_finalized_block: Option<N::BlockResponse>,
    /// Slot of when we last requested the finalized block
    last_finalized_slot: Option<u64>,
    /// Whether we're in polling mode (actively checking for finalized block updates)
    polling_mode: bool,
}

#[cfg(feature = "pubsub")]
impl<N: alloy_network::Network> FinalizedBlocksStream<N> {
    /// Ethereum slot duration in seconds
    const SLOT_DURATION: u64 = alloy_eips::merge::SLOT_DURATION_SECS;
    /// Number of slots to wait before requesting finalized block again
    /// This is approximately 1 epochs (32 slots ≈ 384 seconds ≈ 6.4 minutes)
    const FINALITY_DELAY_SLOTS: u64 = 32;

    /// The timestamp of the genesis block in seconds.
    const GENESIS_TIME: u64 = 1_606_824_023; // Beacon-genesis is 2020-12-01 12:00:23 UTC

    /// Create a new [`FinalizedBlocksStream`] with the given subscription and client.
    pub fn new(
        sub: alloy_pubsub::Subscription<N::HeaderResponse>,
        client: alloy_rpc_client::WeakClient,
        kind: BlockTransactionsKind,
    ) -> Self {
        let inner = Box::pin(sub.into_stream());
        Self {
            inner,
            client,
            kind,
            pending_request: None,
            current_header: None,
            cached_finalized_block: None,
            last_finalized_slot: None,
            polling_mode: false,
        }
    }

    fn global_slot(ts: u64) -> u64 {
        (ts.saturating_sub(Self::GENESIS_TIME)) / Self::SLOT_DURATION
    }

    /// Check if we should enter polling mode or make a finalized block request
    fn should_request_finalized_block(&mut self, current_timestamp: u64) -> bool {
        match self.last_finalized_slot {
            None => {
                // First request - enter polling mode
                self.polling_mode = true;
                tracing::debug!("🔔 first finalized request → enter polling_mode");
                true
            }
            Some(last_slot) => {
                let current_slot = Self::global_slot(current_timestamp);
                let diff = current_slot.saturating_sub(last_slot);

                tracing::trace!(
                    current_slot,
                    last_slot,
                    diff,
                    polling = self.polling_mode,
                    "⏱  evaluating finalized request"
                );

                if current_slot % 32 == 0 && !self.polling_mode {
                    tracing::debug!(current_slot, "🌑 epoch boundary → enter polling_mode");
                    self.polling_mode = true;
                    true
                } else if diff >= Self::FINALITY_DELAY_SLOTS {
                    self.polling_mode = true;
                    true
                } else {
                    self.polling_mode
                }
            }
        }
    }

    /// Check if the new finalized block is different from our cached one
    fn is_finalized_block_updated(&self, new_block: &Option<N::BlockResponse>) -> bool {
        use alloy_network_primitives::BlockResponse;

        match (&self.cached_finalized_block, new_block) {
            (None, Some(_)) => true, // First finalized block
            (Some(cached), Some(new)) => {
                // Compare block numbers to see if it's a newer finalized block
                cached.header().number() != new.header().number()
            }
            (Some(_), None) => false, // New request returned None, keep cached
            (None, None) => false,    // Both None, no update
        }
    }
}

#[cfg(feature = "pubsub")]
impl<N: alloy_network::Network> Stream for FinalizedBlocksStream<N> {
    type Item = TransportResult<(N::HeaderResponse, Option<N::BlockResponse>)>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // If we have a pending request, poll it first
        if let Some(pending) = this.pending_request.as_mut() {
            let result = ready!(pending.as_mut().poll(cx));

            this.pending_request = None;

            let header = this
                .current_header
                .take()
                .expect("current_header should be set when pending_request is set");

            tracing::debug!(header_number = %header.number(), "📬 finalized RPC responded");

            match result {
                Ok(finalized_block) => {
                    let processed_block = finalized_block.map(|block| {
                        if this.kind.is_hashes() {
                            utils::convert_to_hashes(Some(block)).unwrap()
                        } else {
                            block
                        }
                    });

                    // Check if we got a new finalized block
                    if this.is_finalized_block_updated(&processed_block) {
                        // Update cache with new finalized block and exit polling mode
                        if let Some(ref block) = processed_block {
                            this.cached_finalized_block = Some(block.clone());
                        }
                        this.last_finalized_slot = processed_block
                            .as_ref()
                            .map(|b| Self::global_slot(b.header().timestamp()));
                        this.polling_mode = false;
                        tracing::debug!(
                            new_finalized = %processed_block.as_ref()
                                .map(|b| b.header().number())
                                .unwrap_or_default(),
                            "✅ finalized advanced → exit polling_mode"
                        );
                    } else if this.polling_mode {
                        // Still in polling mode but no update, update finalized slot anyway
                        this.last_finalized_slot = Some(Self::global_slot(header.timestamp()));
                        tracing::trace!("🔄 still polling - no finalized update yet");
                        // Keep polling_mode = true to continue polling
                    }

                    return Poll::Ready(Some(Ok((
                        header,
                        processed_block.or_else(|| this.cached_finalized_block.clone()),
                    ))));
                }
                Err(_err) => {
                    // If finalized block request fails, still return the header with cached
                    // block
                    this.last_finalized_slot = Some(Self::global_slot(header.timestamp()));
                    tracing::warn!("❌ finalized RPC failed, using cached block");
                    // Keep polling mode status unchanged on error
                    return Poll::Ready(Some(Ok((header, this.cached_finalized_block.clone()))));
                }
            }
        }

        // Poll the inner stream for the next header
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(header)) => {
                let current_timestamp = header.timestamp();

                // Check if we should request a new finalized block
                if this.should_request_finalized_block(current_timestamp) {
                    let client = match this.client.upgrade() {
                        Some(client) => client,
                        None => {
                            return Poll::Ready(Some(Err(TransportError::local_usage_str(
                                "Client dropped",
                            ))));
                        }
                    };

                    let kind = this.kind;

                    // Create the future for fetching finalized block
                    let call = EthGetBlock::<N::BlockResponse>::by_number(
                        BlockNumberOrTag::Finalized,
                        client.as_ref(),
                    )
                    .kind(kind)
                    .into_future()
                    .boxed();

                    // Store the header and the pending request
                    this.current_header = Some(header);
                    this.pending_request = Some(call);

                    // Poll the future immediately
                    if let Some(pending) = this.pending_request.as_mut() {
                        let result = ready!(pending.as_mut().poll(cx));

                        this.pending_request = None;
                        let header = this.current_header.take().unwrap();

                        match result {
                            Ok(finalized_block) => {
                                let processed_block = finalized_block.map(|block| {
                                    if this.kind.is_hashes() {
                                        utils::convert_to_hashes(Some(block)).unwrap()
                                    } else {
                                        block
                                    }
                                });

                                // Check if we got a new finalized block
                                if this.is_finalized_block_updated(&processed_block) {
                                    // Update cache with new finalized block and exit
                                    // polling mode
                                    if let Some(ref block) = processed_block {
                                        this.cached_finalized_block = Some(block.clone());
                                    }
                                    this.polling_mode = false; // Exit polling mode
                                    tracing::debug!(
                                        new_finalized = %processed_block.as_ref()
                                            .map(|b| b.header().number())
                                            .unwrap_or_default(),
                                        "✅ finalized advanced → exit polling_mode (immediate)"
                                    );
                                } else if this.polling_mode {
                                    // Keep polling mode active if no update
                                    tracing::trace!(
                                        "🔄 still polling - no finalized update yet (immediate)"
                                    );
                                    // polling_mode stays true
                                }
                                this.last_finalized_slot =
                                    Some(Self::global_slot(current_timestamp));

                                Poll::Ready(Some(Ok((
                                    header,
                                    processed_block.or_else(|| this.cached_finalized_block.clone()),
                                ))))
                            }
                            Err(_err) => {
                                // If finalized block request fails, still return the header
                                // with cached block
                                this.last_finalized_slot =
                                    Some(Self::global_slot(current_timestamp));
                                tracing::warn!(
                                    "❌ finalized RPC failed, using cached block (immediate)"
                                );
                                // Keep polling mode status unchanged on error
                                Poll::Ready(Some(Ok((header, this.cached_finalized_block.clone()))))
                            }
                        }
                    } else {
                        Poll::Pending
                    }
                } else {
                    // Use cached finalized block without making a request
                    tracing::trace!(
                        header_number = %header.number(),
                        cached_finalized = %this.cached_finalized_block.as_ref()
                            .map(|b| b.header().number())
                            .unwrap_or_default(),
                        "💾 using cached finalized block"
                    );
                    Poll::Ready(Some(Ok((header, this.cached_finalized_block.clone()))))
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
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
            let err_str = err.to_string();
            if err_str.contains("no response") || err.is_transport_error() {
                // response can be flaky due to network issues
                eprintln!("skipping flaky response: {err:?}");
                return;
            }
        }
        let _block = res.unwrap();
    }
}
