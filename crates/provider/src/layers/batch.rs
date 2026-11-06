use crate::{
    bindings::{ArbSys, IMulticall3},
    utils::{Eip1559Estimation, Eip1559Estimator},
    Caller, EthCall, EthCallMany, EthGetBlock, FilterPollerBuilder, PendingTransaction,
    PendingTransactionBuilder, PendingTransactionConfig, PendingTransactionError, Provider,
    ProviderCall, ProviderLayer, RootProvider, RpcWithBlock, SendableTx, WatchBlocks,
    WatchBlocksFrom, WatchCanonicalBlocksFrom, WatchCanonicalLogsFrom, WatchHeaders, WatchLogsFrom,
    ARB_SYS_ADDRESS, MULTICALL3_ADDRESS,
};
#[cfg(feature = "pubsub")]
use crate::{GetSubscription, SubFullBlocks};
use alloy_eips::{eip7928::BlockAccessList, BlockId, BlockNumberOrTag};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{
    Address, BlockHash, Bytes, StorageKey, StorageValue, TxHash, B256, U128, U256, U64,
};
use alloy_rpc_client::{NoParams, WeakClient};
#[cfg(feature = "pubsub")]
use alloy_rpc_types_eth::pubsub::{Params, SubscriptionKind};
use alloy_rpc_types_eth::{
    erc4337::TransactionConditional,
    simulate::{SimulatePayload, SimulatedBlock},
    AccessListResult, Bundle, EIP1186AccountProofResponse, EthCallResponse, FeeHistory,
    FillTransaction, Filter, FilterChanges, Index, Log, StorageValuesRequest,
    StorageValuesResponse, SyncStatus,
};
use alloy_sol_types::{SolCall, SolType, SolValue};
use alloy_transport::{
    utils::Spawnable, SharedTransportError, TransportErrorKind, TransportResult,
};
use serde_json::value::RawValue;
use std::{borrow::Cow, fmt, future::IntoFuture, marker::PhantomData, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

/// This is chosen somewhat arbitrarily. It should be short enough to not cause a noticeable
/// delay on individual requests, but long enough to allow for batching requests issued together in
/// a short period of time, such as when using `join!` macro or similar future combinators.
const DEFAULT_WAIT: Duration = Duration::from_millis(1);

/// Provider layer that aggregates contract calls (`eth_call`) over a time period into a single
/// [Multicall3] contract call.
///
/// Some methods, such as `eth_getBlockNumber`, are first converted into contract calls to the
/// [Multicall3] contract itself and then aggregated with other `eth_call`s.
///
/// Only calls that:
/// - target the latest block ID,
/// - have no state overrides,
/// - have a target address and calldata,
/// - have no other properties (nonce, gas, etc.)
///
/// can be sent with a multicall. This of course requires that the [Multicall3] contract is
/// deployed on the network, by default at [`MULTICALL3_ADDRESS`].
///
/// This layer is useful for reducing the number of network requests made.
/// However, this only works when requests are made in parallel, for example when using the
/// [`tokio::join!`] macro or in multiple threads/tasks, as otherwise the requests will be sent one
/// by one as normal, but with an added delay.
///
/// This layer intercepts [`Provider::call`], [`Provider::get_balance`],
/// [`Provider::get_block_number`] and [`Provider::get_chain_id`]. Requests sent through the
/// batching backend, including its single-request fallback, target `latest` even if an inner
/// [`BlockIdLayer`](crate::layers::BlockIdLayer) sets a different default.
///
/// Other RPC methods are forwarded to the inner provider, preserving its overrides and default
/// block. An `eth_call` that is not batchable instead goes directly through the root RPC client,
/// preserving explicit call parameters but bypassing the inner provider's `call` override and
/// default block.
///
/// # Examples
///
/// ```no_run
/// use alloy_provider::{layers::CallBatchLayer, Provider, ProviderBuilder};
/// use std::time::Duration;
///
/// # async fn f(url: &str) -> Result<(), Box<dyn std::error::Error>> {
/// // Build a provider with the default call batching configuration.
/// let provider = ProviderBuilder::new().with_call_batching().connect(url).await?;
///
/// // Build a provider with a custom call batching configuration.
/// let provider = ProviderBuilder::new()
///     .layer(CallBatchLayer::new().wait(Duration::from_millis(10)))
///     .connect(url)
///     .await?;
///
/// // Both of these requests will be batched together and only 1 network request will be made.
/// let (block_number_result, chain_id_result) =
///     tokio::join!(provider.get_block_number(), provider.get_chain_id());
/// let block_number = block_number_result?;
/// let chain_id = chain_id_result?;
/// println!("block number: {block_number}, chain id: {chain_id}");
/// # Ok(())
/// # }
/// ```
///
/// [Multicall3]: https://github.com/mds1/multicall3
#[derive(Debug)]
pub struct CallBatchLayer {
    m3a: Address,
    wait: Duration,
    arbsys: bool,
}

impl Default for CallBatchLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl CallBatchLayer {
    /// Create a new `CallBatchLayer` with a default wait of 1ms.
    pub const fn new() -> Self {
        Self { m3a: MULTICALL3_ADDRESS, wait: DEFAULT_WAIT, arbsys: false }
    }

    /// Set the amount of time to wait before sending the batch.
    ///
    /// This is the amount of time to wait after the first request is received before sending all
    /// the requests received in that time period.
    ///
    /// This means that every request has a maximum delay of `wait` before being sent.
    ///
    /// The default is 1ms.
    pub const fn wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Set the multicall3 address.
    ///
    /// The default is [`MULTICALL3_ADDRESS`].
    pub const fn multicall3_address(mut self, m3a: Address) -> Self {
        self.m3a = m3a;
        self
    }

    /// Use the Arbitrum `ArbSys` precompile for block number queries.
    ///
    /// On Arbitrum, `block.number` returns the parent chain’s block number (L1).
    /// Without this setting, batched `eth_blockNumber` calls through Multicall3
    /// will therefore return the wrong value. Enabling this queries the L2 block
    /// number via `ArbSys` instead.
    ///
    /// The default is `false`.
    /// This should only be enabled when interacting with Arbitrum rollups.
    pub const fn arbitrum_compat(mut self) -> Self {
        self.arbsys = true;
        self
    }
}

impl<P, N> ProviderLayer<P, N> for CallBatchLayer
where
    P: Provider<N> + 'static,
    N: Network,
{
    type Provider = CallBatchProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        CallBatchProvider::new(inner, self)
    }
}

type CallBatchMsgTx = TransportResult<IMulticall3::Result>;

struct CallBatchMsg<N: Network> {
    kind: CallBatchMsgKind<N>,
    tx: oneshot::Sender<CallBatchMsgTx>,
}

impl<N: Network> fmt::Debug for CallBatchMsg<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BatchProviderMessage(")?;
        self.kind.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Debug)]
enum CallBatchMsgKind<N: Network = Ethereum> {
    Call(N::TransactionRequest),
    BlockNumber,
    ChainId,
    Balance(Address),
}

impl<N: Network> CallBatchMsg<N> {
    fn new(kind: CallBatchMsgKind<N>) -> (Self, oneshot::Receiver<CallBatchMsgTx>) {
        let (tx, rx) = oneshot::channel();
        (Self { kind, tx }, rx)
    }
}

impl<N: Network> CallBatchMsgKind<N> {
    fn to_call3(&self, m3a: Address, arbsys: bool) -> IMulticall3::Call3 {
        let m3a_call = |data: Vec<u8>| IMulticall3::Call3 {
            target: m3a,
            allowFailure: true,
            callData: data.into(),
        };
        match self {
            Self::Call(tx) => IMulticall3::Call3 {
                target: tx.to().unwrap_or_default(),
                allowFailure: true,
                callData: tx.input().cloned().unwrap_or_default(),
            },
            Self::BlockNumber => {
                if arbsys {
                    return IMulticall3::Call3 {
                        target: ARB_SYS_ADDRESS,
                        allowFailure: false,
                        callData: ArbSys::arbBlockNumberCall {}.abi_encode().into(),
                    };
                }
                m3a_call(IMulticall3::getBlockNumberCall {}.abi_encode())
            }
            Self::ChainId => m3a_call(IMulticall3::getChainIdCall {}.abi_encode()),
            &Self::Balance(addr) => m3a_call(IMulticall3::getEthBalanceCall { addr }.abi_encode()),
        }
    }
}

/// A provider that batches multiple requests into a single request.
///
/// Ordinary RPC methods delegate to the inner provider to preserve its overrides.
///
/// See [`CallBatchLayer`] for more information.
pub struct CallBatchProvider<P, N: Network = Ethereum> {
    provider: Arc<P>,
    inner: CallBatchProviderInner<N>,
    _pd: PhantomData<N>,
}

impl<P, N: Network> Clone for CallBatchProvider<P, N> {
    fn clone(&self) -> Self {
        Self { provider: self.provider.clone(), inner: self.inner.clone(), _pd: PhantomData }
    }
}

impl<P: fmt::Debug, N: Network> fmt::Debug for CallBatchProvider<P, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BatchProvider(")?;
        self.provider.fmt(f)?;
        f.write_str(")")
    }
}

impl<P: Provider<N> + 'static, N: Network> CallBatchProvider<P, N> {
    fn new(inner: P, layer: &CallBatchLayer) -> Self {
        let inner = Arc::new(inner);
        let tx = CallBatchBackend::spawn(inner.clone(), layer);
        Self { provider: inner, inner: CallBatchProviderInner { tx }, _pd: PhantomData }
    }
}

#[derive(Clone)]
struct CallBatchProviderInner<N: Network> {
    tx: mpsc::UnboundedSender<CallBatchMsg<N>>,
}

/// Returns `true` if the serialized transaction request can be represented as a Multicall3
/// [`Call3`](IMulticall3::Call3), i.e. it is an object made up of nothing but `to`, `data` and
/// `input`.
///
/// Any other shape is rejected. A `Call3` can only carry a target and calldata, so a request we
/// cannot prove fits that shape must be sent as a regular `eth_call` rather than being silently
/// stripped of its remaining fields when the multicall is built.
fn is_multicall_compatible(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(obj) => {
            obj.keys().all(|k| matches!(k.as_str(), "to" | "data" | "input"))
        }
        _ => false,
    }
}

impl<N: Network> CallBatchProviderInner<N> {
    /// We only want to perform a scheduled multicall if:
    /// - The request has no block ID or state overrides,
    /// - The request has a target address,
    /// - The request can be represented as a Multicall3 `Call3`.
    ///
    /// A `Call3` carries nothing but a target and calldata, so requests carrying any other property
    /// (`nonce`, `gas`, etc.) must be sent as a regular `eth_call`. Requests we cannot inspect at
    /// all are rejected too: falling back to a plain `eth_call` is always correct, batching them is
    /// not.
    /// Ref: <https://github.com/wevm/viem/blob/ba8319f71503af8033fd3c77cfb64c7eb235c6a9/src/actions/public/call.ts#L295>
    fn should_batch_call(&self, params: &crate::EthCallParams<N>) -> bool {
        // If a specific block ID is requested (not "latest"), skip batching
        // as multicall only works reliably on the latest block.
        if params.block().is_some_and(|block| block != BlockId::latest()) {
            return false;
        }
        if params.overrides.as_ref().is_some_and(|overrides| !overrides.is_empty()) {
            return false;
        }
        if params.block_overrides().is_some_and(|overrides| !overrides.is_empty()) {
            return false;
        }
        let tx = params.data();
        if tx.to().is_none() {
            return false;
        }
        // A `Call3` can only carry a target and calldata, so only batch requests that are known to
        // fit that shape. Anything that cannot be inspected (including a request that fails to
        // serialize) falls back to a regular `eth_call`, which is always correct.
        if !serde_json::to_value(tx).is_ok_and(|value| is_multicall_compatible(&value)) {
            return false;
        }
        true
    }

    async fn schedule(self, msg: CallBatchMsgKind<N>) -> TransportResult<Bytes> {
        let (msg, rx) = CallBatchMsg::new(msg);
        self.tx.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;

        let IMulticall3::Result { success, returnData } =
            rx.await.map_err(|_| TransportErrorKind::backend_gone())??;

        if !success {
            let revert_data = if returnData.is_empty() {
                "".to_string()
            } else {
                format!(" with data: {returnData}")
            };
            Err(TransportErrorKind::custom_str(&format!(
                "multicall batched call reverted{revert_data}"
            )))
        } else {
            Ok(returnData)
        }
    }

    async fn schedule_and_decode<T>(self, msg: CallBatchMsgKind<N>) -> TransportResult<T>
    where
        T: SolValue + From<<T::SolType as SolType>::RustType>,
    {
        let data = self.schedule(msg).await?;
        T::abi_decode(&data).map_err(TransportErrorKind::custom)
    }
}

struct CallBatchBackend<P, N: Network = Ethereum> {
    inner: Arc<P>,
    m3a: Address,
    wait: Duration,
    arbsys: bool,
    rx: mpsc::UnboundedReceiver<CallBatchMsg<N>>,
    pending: Vec<CallBatchMsg<N>>,
}

impl<P: Provider<N> + 'static, N: Network> CallBatchBackend<P, N> {
    fn spawn(inner: Arc<P>, layer: &CallBatchLayer) -> mpsc::UnboundedSender<CallBatchMsg<N>> {
        let CallBatchLayer { m3a, wait, arbsys } = *layer;
        let (tx, rx) = mpsc::unbounded_channel();
        let this = Self { inner, m3a, wait, arbsys, rx, pending: Vec::new() };
        this.run().spawn_task();
        tx
    }

    async fn run(mut self) {
        'outer: loop {
            // Wait for the first message.
            debug_assert!(self.pending.is_empty());
            match self.rx.recv().await {
                Some(msg) => self.process_msg(msg),
                None => break,
            }

            // Handle all remaining messages after waiting the duration.
            debug_assert!(!self.pending.is_empty());
            sleep(self.wait).await;
            'inner: loop {
                match self.rx.try_recv() {
                    Ok(msg) => self.process_msg(msg),
                    Err(mpsc::error::TryRecvError::Empty) => break 'inner,
                    Err(mpsc::error::TryRecvError::Disconnected) => break 'outer,
                }
            }
            // No more messages, send the batch.
            self.send_batch().await;
        }
    }

    fn process_msg(&mut self, msg: CallBatchMsg<N>) {
        self.pending.push(msg);
    }

    async fn send_batch(&mut self) {
        let mut pending = std::mem::take(&mut self.pending);

        // Remove requests where the client has disconnected.
        pending.retain(|msg| !msg.tx.is_closed());

        // If all clients disconnected, return early.
        if pending.is_empty() {
            return;
        }

        // If there's only a single call, avoid batching and perform the request directly.
        if pending.len() == 1 {
            let msg = pending.into_iter().next().unwrap();
            let result = self.call_one(msg.kind).await;
            let _ = msg.tx.send(result);
            return;
        }

        let result = self.send_batch_inner(&pending).await;
        match result {
            Ok(results) => {
                if results.len() != pending.len() {
                    let err = format!(
                        "multicall batch response count mismatch: expected {}, got {}",
                        pending.len(),
                        results.len()
                    );
                    for msg in pending {
                        let _ = msg.tx.send(Err(TransportErrorKind::custom_str(&err)));
                    }
                    return;
                }

                for (result, msg) in results.into_iter().zip(pending) {
                    let _ = msg.tx.send(Ok(result));
                }
            }
            Err(e) => {
                let error = SharedTransportError::new(e);
                for msg in pending {
                    let _ = msg.tx.send(Err(error.to_error()));
                }
            }
        }
    }

    async fn call_one(&mut self, msg: CallBatchMsgKind<N>) -> TransportResult<IMulticall3::Result> {
        let m3_res =
            |success, return_data| IMulticall3::Result { success, returnData: return_data };
        match msg {
            CallBatchMsgKind::Call(tx) => {
                self.inner.call(tx).latest().await.map(|res| m3_res(true, res))
            }
            CallBatchMsgKind::BlockNumber => {
                self.inner.get_block_number().await.map(|res| m3_res(true, res.abi_encode().into()))
            }
            CallBatchMsgKind::ChainId => {
                self.inner.get_chain_id().await.map(|res| m3_res(true, res.abi_encode().into()))
            }
            CallBatchMsgKind::Balance(addr) => self
                .inner
                .get_balance(addr)
                .latest()
                .await
                .map(|res| m3_res(true, res.abi_encode().into())),
        }
    }

    async fn send_batch_inner(
        &self,
        pending: &[CallBatchMsg<N>],
    ) -> TransportResult<Vec<IMulticall3::Result>> {
        let calls: Vec<_> =
            pending.iter().map(|msg| msg.kind.to_call3(self.m3a, self.arbsys)).collect();

        let tx = N::TransactionRequest::default()
            .with_to(self.m3a)
            .with_input(IMulticall3::aggregate3Call { calls }.abi_encode());

        // Only calls targeting the latest block are batched (see `should_batch_call`), so pin the
        // aggregate call to it instead of inheriting the `pending` default of `Provider::call`.
        let bytes = self.inner.call(tx).latest().await?;
        if bytes.is_empty() {
            return Err(TransportErrorKind::custom_str(&format!(
                "Multicall3 not deployed at {}",
                self.m3a
            )));
        }

        let ret = IMulticall3::aggregate3Call::abi_decode_returns(&bytes)
            .map_err(TransportErrorKind::custom)?;
        Ok(ret)
    }
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<P: Provider<N> + 'static, N: Network> Provider<N> for CallBatchProvider<P, N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }

    fn get_accounts(&self) -> ProviderCall<NoParams, Vec<Address>> {
        self.provider.get_accounts()
    }

    fn get_blob_base_fee(&self) -> ProviderCall<NoParams, U128, u128> {
        self.provider.get_blob_base_fee()
    }

    fn get_block_number(
        &self,
    ) -> crate::ProviderCall<
        alloy_rpc_client::NoParams,
        alloy_primitives::U64,
        alloy_primitives::BlockNumber,
    > {
        crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule_and_decode::<u64>(CallBatchMsgKind::BlockNumber),
        ))
    }

    fn call(&self, tx: <N as Network>::TransactionRequest) -> crate::EthCall<N, Bytes> {
        crate::EthCall::call(CallBatchCaller::new(self), tx)
    }

    fn call_many<'req>(
        &self,
        bundles: &'req [Bundle],
    ) -> EthCallMany<'req, N, Vec<Vec<EthCallResponse>>> {
        self.provider.call_many(bundles)
    }

    fn simulate<'req>(
        &self,
        payload: &'req SimulatePayload,
    ) -> RpcWithBlock<&'req SimulatePayload, Vec<SimulatedBlock<N::BlockResponse>>> {
        self.provider.simulate(payload)
    }

    fn get_chain_id(
        &self,
    ) -> crate::ProviderCall<
        alloy_rpc_client::NoParams,
        alloy_primitives::U64,
        alloy_primitives::ChainId,
    > {
        crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule_and_decode::<u64>(CallBatchMsgKind::ChainId),
        ))
    }

    fn create_access_list<'a>(
        &self,
        request: &'a N::TransactionRequest,
    ) -> RpcWithBlock<&'a N::TransactionRequest, AccessListResult> {
        self.provider.create_access_list(request)
    }

    fn estimate_gas(&self, tx: N::TransactionRequest) -> EthCall<N, U64, u64> {
        self.provider.estimate_gas(tx)
    }

    async fn estimate_eip1559_fees_with(
        &self,
        estimator: Eip1559Estimator,
    ) -> TransportResult<Eip1559Estimation> {
        self.provider.estimate_eip1559_fees_with(estimator).await
    }

    async fn estimate_eip1559_fees(&self) -> TransportResult<Eip1559Estimation> {
        self.provider.estimate_eip1559_fees().await
    }

    async fn get_fee_history(
        &self,
        block_count: u64,
        last_block: BlockNumberOrTag,
        reward_percentiles: &[f64],
    ) -> TransportResult<FeeHistory> {
        self.provider.get_fee_history(block_count, last_block, reward_percentiles).await
    }

    fn get_gas_price(&self) -> ProviderCall<NoParams, U128, u128> {
        self.provider.get_gas_price()
    }

    fn get_account_info(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, alloy_rpc_types_eth::AccountInfo> {
        self.provider.get_account_info(address)
    }

    fn get_account(&self, address: Address) -> RpcWithBlock<Address, alloy_consensus::TrieAccount> {
        self.provider.get_account(address)
    }

    fn get_balance(&self, address: Address) -> crate::RpcWithBlock<Address, U256, U256> {
        let this = self.clone();
        crate::RpcWithBlock::new_provider(move |block| {
            if block != BlockId::latest() {
                this.provider.get_balance(address).block_id(block).into_future()
            } else {
                ProviderCall::BoxedFuture(Box::pin(
                    this.inner
                        .clone()
                        .schedule_and_decode::<U256>(CallBatchMsgKind::Balance(address)),
                ))
            }
        })
    }

    fn get_block(&self, block: BlockId) -> EthGetBlock<N::BlockResponse> {
        self.provider.get_block(block)
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> EthGetBlock<N::BlockResponse> {
        self.provider.get_block_by_hash(hash)
    }

    fn get_block_by_number(&self, number: BlockNumberOrTag) -> EthGetBlock<N::BlockResponse> {
        self.provider.get_block_by_number(number)
    }

    async fn get_block_transaction_count_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<u64>> {
        self.provider.get_block_transaction_count_by_hash(hash).await
    }

    async fn get_block_transaction_count_by_number(
        &self,
        block_number: BlockNumberOrTag,
    ) -> TransportResult<Option<u64>> {
        self.provider.get_block_transaction_count_by_number(block_number).await
    }

    fn get_block_receipts(
        &self,
        block: BlockId,
    ) -> ProviderCall<(BlockId,), Option<Vec<N::ReceiptResponse>>> {
        self.provider.get_block_receipts(block)
    }

    async fn get_block_access_list(
        &self,
        block: BlockId,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.provider.get_block_access_list(block).await
    }

    async fn get_block_access_list_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.provider.get_block_access_list_by_hash(hash).await
    }

    async fn get_block_access_list_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<BlockAccessList>> {
        self.provider.get_block_access_list_by_number(number).await
    }

    async fn get_block_access_list_raw(&self, block: BlockId) -> TransportResult<Option<Bytes>> {
        self.provider.get_block_access_list_raw(block).await
    }

    async fn get_header(&self, block: BlockId) -> TransportResult<Option<N::HeaderResponse>> {
        self.provider.get_header(block).await
    }

    async fn get_header_by_hash(
        &self,
        hash: BlockHash,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.provider.get_header_by_hash(hash).await
    }

    async fn get_header_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> TransportResult<Option<N::HeaderResponse>> {
        self.provider.get_header_by_number(number).await
    }

    fn get_code_at(&self, address: Address) -> crate::RpcWithBlock<Address, Bytes> {
        self.provider.get_code_at(address)
    }

    async fn watch_blocks(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.provider.watch_blocks().await
    }

    async fn watch_full_blocks(&self) -> TransportResult<WatchBlocks<N::BlockResponse>> {
        self.provider.watch_full_blocks().await
    }

    async fn watch_headers(&self) -> TransportResult<WatchHeaders<N::HeaderResponse>> {
        self.provider.watch_headers().await
    }

    async fn watch_pending_transactions(&self) -> TransportResult<FilterPollerBuilder<B256>> {
        self.provider.watch_pending_transactions().await
    }

    async fn watch_logs(&self, filter: &Filter) -> TransportResult<FilterPollerBuilder<Log>> {
        self.provider.watch_logs(filter).await
    }

    fn watch_blocks_from(&self, start_block: u64) -> WatchBlocksFrom<N> {
        self.provider.watch_blocks_from(start_block)
    }

    fn watch_canonical_blocks_from(&self, start_block: u64) -> WatchCanonicalBlocksFrom<N> {
        self.provider.watch_canonical_blocks_from(start_block)
    }

    fn watch_logs_from(&self, start_block: u64, filter: &Filter) -> WatchLogsFrom<N> {
        self.provider.watch_logs_from(start_block, filter)
    }

    fn watch_canonical_logs_from(
        &self,
        start_block: u64,
        filter: &Filter,
    ) -> WatchCanonicalLogsFrom<N> {
        self.provider.watch_canonical_logs_from(start_block, filter)
    }

    async fn watch_full_pending_transactions(
        &self,
    ) -> TransportResult<FilterPollerBuilder<N::TransactionResponse>> {
        self.provider.watch_full_pending_transactions().await
    }

    async fn get_filter_changes<R: RpcRecv>(&self, id: U256) -> TransportResult<Vec<R>>
    where
        Self: Sized,
    {
        // Arc's auto_impl retains the trait default for generic methods; call P directly.
        self.provider.as_ref().get_filter_changes(id).await
    }

    async fn get_filter_changes_dyn(&self, id: U256) -> TransportResult<FilterChanges> {
        self.provider.get_filter_changes_dyn(id).await
    }

    async fn get_filter_logs(&self, id: U256) -> TransportResult<Vec<Log>> {
        self.provider.get_filter_logs(id).await
    }

    async fn uninstall_filter(&self, id: U256) -> TransportResult<bool> {
        self.provider.uninstall_filter(id).await
    }

    async fn watch_pending_transaction(
        &self,
        config: PendingTransactionConfig,
    ) -> Result<PendingTransaction, PendingTransactionError> {
        self.provider.watch_pending_transaction(config).await
    }

    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        self.provider.get_logs(filter).await
    }

    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<(Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        self.provider.get_proof(address, keys)
    }

    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<(Address, U256), StorageValue> {
        self.provider.get_storage_at(address, key)
    }

    fn get_storage_values(
        &self,
        requests: StorageValuesRequest,
    ) -> RpcWithBlock<(StorageValuesRequest,), StorageValuesResponse> {
        self.provider.get_storage_values(requests)
    }

    fn get_transaction_by_sender_nonce(
        &self,
        sender: Address,
        nonce: u64,
    ) -> ProviderCall<(Address, U64), Option<N::TransactionResponse>> {
        self.provider.get_transaction_by_sender_nonce(sender, nonce)
    }

    fn get_transaction_by_hash(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::TransactionResponse>> {
        self.provider.get_transaction_by_hash(hash)
    }

    fn get_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<N::TransactionResponse>> {
        self.provider.get_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_raw_transaction_by_block_hash_and_index(
        &self,
        block_hash: B256,
        index: usize,
    ) -> ProviderCall<(B256, Index), Option<Bytes>> {
        self.provider.get_raw_transaction_by_block_hash_and_index(block_hash, index)
    }

    fn get_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<N::TransactionResponse>> {
        self.provider.get_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: usize,
    ) -> ProviderCall<(BlockNumberOrTag, Index), Option<Bytes>> {
        self.provider.get_raw_transaction_by_block_number_and_index(block_number, index)
    }

    fn get_raw_transaction_by_hash(&self, hash: TxHash) -> ProviderCall<(TxHash,), Option<Bytes>> {
        self.provider.get_raw_transaction_by_hash(hash)
    }

    fn get_transaction_count(
        &self,
        address: Address,
    ) -> RpcWithBlock<Address, U64, u64, fn(U64) -> u64> {
        self.provider.get_transaction_count(address)
    }

    fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> ProviderCall<(TxHash,), Option<N::ReceiptResponse>> {
        self.provider.get_transaction_receipt(hash)
    }

    async fn get_uncle(&self, tag: BlockId, idx: u64) -> TransportResult<Option<N::BlockResponse>> {
        self.provider.get_uncle(tag, idx).await
    }

    async fn get_uncle_count(&self, tag: BlockId) -> TransportResult<u64> {
        self.provider.get_uncle_count(tag).await
    }

    fn get_max_priority_fee_per_gas(&self) -> ProviderCall<NoParams, U128, u128> {
        self.provider.get_max_priority_fee_per_gas()
    }

    async fn new_block_filter(&self) -> TransportResult<U256> {
        self.provider.new_block_filter().await
    }

    async fn new_filter(&self, filter: &Filter) -> TransportResult<U256> {
        self.provider.new_filter(filter).await
    }

    async fn new_pending_transactions_filter(&self, full: bool) -> TransportResult<U256> {
        self.provider.new_pending_transactions_filter(full).await
    }

    async fn send_raw_transaction(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.provider.send_raw_transaction(encoded_tx).await
    }

    async fn send_raw_transaction_sync(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<N::ReceiptResponse> {
        self.provider.send_raw_transaction_sync(encoded_tx).await
    }

    async fn send_raw_transaction_conditional(
        &self,
        encoded_tx: &[u8],
        conditional: TransactionConditional,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.provider.send_raw_transaction_conditional(encoded_tx, conditional).await
    }

    async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.provider.send_transaction(tx).await
    }

    async fn send_tx_envelope(
        &self,
        tx: N::TxEnvelope,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.provider.send_tx_envelope(tx).await
    }

    async fn send_transaction_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.provider.send_transaction_internal(tx).await
    }

    async fn send_transaction_sync(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::ReceiptResponse> {
        self.provider.send_transaction_sync(tx).await
    }

    async fn send_transaction_sync_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<N::ReceiptResponse> {
        self.provider.send_transaction_sync_internal(tx).await
    }

    async fn sign_transaction(&self, tx: N::TransactionRequest) -> TransportResult<Bytes> {
        self.provider.sign_transaction(tx).await
    }

    async fn fill_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<FillTransaction<N::TxEnvelope>>
    where
        N::TxEnvelope: RpcRecv,
    {
        self.provider.fill_transaction(tx).await
    }

    async fn fill_and_sign_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::TxEnvelope> {
        self.provider.fill_and_sign_transaction(tx).await
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_blocks(&self) -> GetSubscription<(SubscriptionKind,), N::HeaderResponse> {
        self.provider.subscribe_blocks()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_full_blocks(&self) -> SubFullBlocks<N> {
        self.provider.subscribe_full_blocks()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_pending_transactions(&self) -> GetSubscription<(SubscriptionKind,), B256> {
        self.provider.subscribe_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_full_pending_transactions(
        &self,
    ) -> GetSubscription<(SubscriptionKind, Params), N::TransactionResponse> {
        self.provider.subscribe_full_pending_transactions()
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_logs(&self, filter: &Filter) -> GetSubscription<(SubscriptionKind, Params), Log> {
        self.provider.subscribe_logs(filter)
    }

    #[cfg(feature = "pubsub")]
    fn subscribe<Params, R>(&self, params: Params) -> GetSubscription<Params, R>
    where
        Params: RpcSend,
        R: RpcRecv,
        Self: Sized,
    {
        self.provider.as_ref().subscribe(params)
    }

    #[cfg(feature = "pubsub")]
    fn subscribe_to<R>(&self, method: &'static str) -> GetSubscription<NoParams, R>
    where
        R: RpcRecv,
        Self: Sized,
    {
        self.provider.as_ref().subscribe_to(method)
    }

    #[cfg(feature = "pubsub")]
    async fn unsubscribe(&self, id: B256) -> TransportResult<()> {
        self.provider.unsubscribe(id).await
    }

    fn syncing(&self) -> ProviderCall<NoParams, SyncStatus> {
        self.provider.syncing()
    }

    fn get_client_version(&self) -> ProviderCall<NoParams, String> {
        self.provider.get_client_version()
    }

    fn get_sha3(&self, data: &[u8]) -> ProviderCall<(String,), B256> {
        self.provider.get_sha3(data)
    }

    fn get_net_version(&self) -> ProviderCall<NoParams, U64, u64> {
        self.provider.get_net_version()
    }

    async fn raw_request<Params, R>(
        &self,
        method: Cow<'static, str>,
        params: Params,
    ) -> TransportResult<R>
    where
        Params: RpcSend,
        R: RpcRecv,
        Self: Sized,
    {
        self.provider.as_ref().raw_request(method, params).await
    }

    async fn raw_request_dyn(
        &self,
        method: Cow<'static, str>,
        params: &RawValue,
    ) -> TransportResult<Box<RawValue>> {
        self.provider.raw_request_dyn(method, params).await
    }

    fn transaction_request(&self) -> N::TransactionRequest {
        self.provider.transaction_request()
    }
}

struct CallBatchCaller<N: Network> {
    inner: CallBatchProviderInner<N>,
    weak: WeakClient,
}

impl<N: Network> CallBatchCaller<N> {
    fn new<P: Provider<N>>(provider: &CallBatchProvider<P, N>) -> Self {
        Self { inner: provider.inner.clone(), weak: provider.provider.weak_client() }
    }
}

impl<N: Network> Caller<N, Bytes> for CallBatchCaller<N> {
    fn call(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Bytes>> {
        if !self.inner.should_batch_call(&params) {
            return Caller::<N, Bytes>::call(&self.weak, params);
        }

        Ok(crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule(CallBatchMsgKind::Call(params.into_data())),
        )))
    }

    fn estimate_gas(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Bytes>> {
        Caller::<N, Bytes>::estimate_gas(&self.weak, params)
    }

    fn call_many(
        &self,
        params: crate::EthCallManyParams<'_>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallManyParams<'static>, Bytes>> {
        Caller::<N, Bytes>::call_many(&self.weak, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layers::{BlockIdLayer, CacheLayer},
        ProviderBuilder,
    };
    use alloy_network::{Ethereum, TransactionBuilder};
    use alloy_primitives::{address, hex};
    use alloy_rpc_types_eth::{BlockOverrides, TransactionRequest};
    use alloy_transport::mock::Asserter;

    // https://etherscan.io/address/0xcA11bde05977b3631167028862bE2a173976CA11#code
    const MULTICALL3_DEPLOYED_CODE: &[u8] = &hex!("0x6080604052600436106100f35760003560e01c80634d2301cc1161008a578063a8b0574e11610059578063a8b0574e1461025a578063bce38bd714610275578063c3077fa914610288578063ee82ac5e1461029b57600080fd5b80634d2301cc146101ec57806372425d9d1461022157806382ad56cb1461023457806386d516e81461024757600080fd5b80633408e470116100c65780633408e47014610191578063399542e9146101a45780633e64a696146101c657806342cbb15c146101d957600080fd5b80630f28c97d146100f8578063174dea711461011a578063252dba421461013a57806327e86d6e1461015b575b600080fd5b34801561010457600080fd5b50425b6040519081526020015b60405180910390f35b61012d610128366004610a85565b6102ba565b6040516101119190610bbe565b61014d610148366004610a85565b6104ef565b604051610111929190610bd8565b34801561016757600080fd5b50437fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0140610107565b34801561019d57600080fd5b5046610107565b6101b76101b2366004610c60565b610690565b60405161011193929190610cba565b3480156101d257600080fd5b5048610107565b3480156101e557600080fd5b5043610107565b3480156101f857600080fd5b50610107610207366004610ce2565b73ffffffffffffffffffffffffffffffffffffffff163190565b34801561022d57600080fd5b5044610107565b61012d610242366004610a85565b6106ab565b34801561025357600080fd5b5045610107565b34801561026657600080fd5b50604051418152602001610111565b61012d610283366004610c60565b61085a565b6101b7610296366004610a85565b610a1a565b3480156102a757600080fd5b506101076102b6366004610d18565b4090565b60606000828067ffffffffffffffff8111156102d8576102d8610d31565b60405190808252806020026020018201604052801561031e57816020015b6040805180820190915260008152606060208201528152602001906001900390816102f65790505b5092503660005b8281101561047757600085828151811061034157610341610d60565b6020026020010151905087878381811061035d5761035d610d60565b905060200281019061036f9190610d8f565b6040810135958601959093506103886020850185610ce2565b73ffffffffffffffffffffffffffffffffffffffff16816103ac6060870187610dcd565b6040516103ba929190610e32565b60006040518083038185875af1925050503d80600081146103f7576040519150601f19603f3d011682016040523d82523d6000602084013e6103fc565b606091505b50602080850191909152901515808452908501351761046d577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260846000fd5b5050600101610325565b508234146104e6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601a60248201527f4d756c746963616c6c333a2076616c7565206d69736d6174636800000000000060448201526064015b60405180910390fd5b50505092915050565b436060828067ffffffffffffffff81111561050c5761050c610d31565b60405190808252806020026020018201604052801561053f57816020015b606081526020019060019003908161052a5790505b5091503660005b8281101561068657600087878381811061056257610562610d60565b90506020028101906105749190610e42565b92506105836020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166105a66020850185610dcd565b6040516105b4929190610e32565b6000604051808303816000865af19150503d80600081146105f1576040519150601f19603f3d011682016040523d82523d6000602084013e6105f6565b606091505b5086848151811061060957610609610d60565b602090810291909101015290508061067d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b50600101610546565b5050509250929050565b43804060606106a086868661085a565b905093509350939050565b6060818067ffffffffffffffff8111156106c7576106c7610d31565b60405190808252806020026020018201604052801561070d57816020015b6040805180820190915260008152606060208201528152602001906001900390816106e55790505b5091503660005b828110156104e657600084828151811061073057610730610d60565b6020026020010151905086868381811061074c5761074c610d60565b905060200281019061075e9190610e76565b925061076d6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166107906040850185610dcd565b60405161079e929190610e32565b6000604051808303816000865af19150503d80600081146107db576040519150601f19603f3d011682016040523d82523d6000602084013e6107e0565b606091505b506020808401919091529015158083529084013517610851577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260646000fd5b50600101610714565b6060818067ffffffffffffffff81111561087657610876610d31565b6040519080825280602002602001820160405280156108bc57816020015b6040805180820190915260008152606060208201528152602001906001900390816108945790505b5091503660005b82811015610a105760008482815181106108df576108df610d60565b602002602001015190508686838181106108fb576108fb610d60565b905060200281019061090d9190610e42565b925061091c6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff1661093f6020850185610dcd565b60405161094d929190610e32565b6000604051808303816000865af19150503d806000811461098a576040519150601f19603f3d011682016040523d82523d6000602084013e61098f565b606091505b506020830152151581528715610a07578051610a07576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b506001016108c3565b5050509392505050565b6000806060610a2b60018686610690565b919790965090945092505050565b60008083601f840112610a4b57600080fd5b50813567ffffffffffffffff811115610a6357600080fd5b6020830191508360208260051b8501011115610a7e57600080fd5b9250929050565b60008060208385031215610a9857600080fd5b823567ffffffffffffffff811115610aaf57600080fd5b610abb85828601610a39565b90969095509350505050565b6000815180845260005b81811015610aed57602081850181015186830182015201610ad1565b81811115610aff576000602083870101525b50601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169290920160200192915050565b600082825180855260208086019550808260051b84010181860160005b84811015610bb1578583037fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe001895281518051151584528401516040858501819052610b9d81860183610ac7565b9a86019a9450505090830190600101610b4f565b5090979650505050505050565b602081526000610bd16020830184610b32565b9392505050565b600060408201848352602060408185015281855180845260608601915060608160051b870101935082870160005b82811015610c52577fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa0888703018452610c40868351610ac7565b95509284019290840190600101610c06565b509398975050505050505050565b600080600060408486031215610c7557600080fd5b83358015158114610c8557600080fd5b9250602084013567ffffffffffffffff811115610ca157600080fd5b610cad86828701610a39565b9497909650939450505050565b838152826020820152606060408201526000610cd96060830184610b32565b95945050505050565b600060208284031215610cf457600080fd5b813573ffffffffffffffffffffffffffffffffffffffff81168114610bd157600080fd5b600060208284031215610d2a57600080fd5b5035919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052603260045260246000fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff81833603018112610dc357600080fd5b9190910192915050565b60008083357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe1843603018112610e0257600080fd5b83018035915067ffffffffffffffff821115610e1d57600080fd5b602001915036819003821315610a7e57600080fd5b8183823760009101908152919050565b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc1833603018112610dc357600080fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa1833603018112610dc357600080fdfea2646970667358221220bb2b5c71a328032f97c676ae39a1ec2148d3e5d6f73d95e9b17910152d61f16264736f6c634300080c0033");
    const COUNTER_ADDRESS: Address = address!("0x1234123412341234123412341234123412341234");
    const COUNTER_DEPLOYED_CODE: &[u8] = &hex!("0x6080604052348015600e575f5ffd5b5060043610603a575f3560e01c80633fb5c1cb14603e5780638381f58a14604f578063d09de08a146068575b5f5ffd5b604d6049366004607d565b5f55565b005b60565f5481565b60405190815260200160405180910390f35b604d5f805490806076836093565b9190505550565b5f60208284031215608c575f5ffd5b5035919050565b5f6001820160af57634e487b7160e01b5f52601160045260245ffd5b506001019056fea2646970667358221220f423ff7a9a85bf49c3769164d3bd24403940510478df27a6b1deac980db69e5664736f6c634300081b0033");

    fn push_m3_success(asserter: &Asserter, returns: &[(bool, Vec<u8>)]) {
        asserter.push_success(
            &returns
                .iter()
                .map(|&(success, ref data)| IMulticall3::Result {
                    success,
                    returnData: Bytes::copy_from_slice(data),
                })
                .collect::<Vec<_>>()
                .abi_encode(),
        )
    }

    #[tokio::test]
    async fn latest_block_number_by_id_is_batched() {
        let asserter = Asserter::new();
        // Answer both request shapes so a batching regression fails on the captured requests,
        // rather than on response decoding.
        let transport = tower::ServiceBuilder::new()
            .map_request({
                let asserter = asserter.clone();
                move |request: alloy_json_rpc::RequestPacket| {
                    match request.as_single().unwrap().method() {
                        "eth_call" => push_m3_success(
                            &asserter,
                            &[(true, 123u64.abi_encode()), (true, 1u64.abi_encode())],
                        ),
                        "eth_blockNumber" => asserter.push_success(&U64::from(123)),
                        "eth_chainId" => asserter.push_success(&U64::from(1)),
                        method => panic!("unexpected RPC method: {method}"),
                    }
                    request
                }
            })
            .service(alloy_transport::mock::MockTransport::new(asserter.clone()));
        let root: RootProvider = ProviderBuilder::default()
            .connect_client(alloy_rpc_client::RpcClient::new(transport, true));
        let provider = CallBatchLayer::new().layer(root);

        let (block_number, chain_id) = tokio::join!(
            provider.get_block_number_by_id(BlockId::latest()),
            provider.get_chain_id(),
        );
        assert_eq!(block_number.unwrap(), Some(123));
        assert_eq!(chain_id.unwrap(), 1);

        let requests = asserter.take_requests();
        let methods: Vec<_> = requests.iter().map(|request| request.method()).collect();
        assert_eq!(requests.len(), 1, "unexpected RPC methods: {methods:?}");
        assert_eq!(requests[0].method(), "eth_call");
        let (tx, block): (TransactionRequest, BlockId) =
            serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
        assert_eq!(block, BlockId::latest());
        assert_eq!(tx.to, Some(MULTICALL3_ADDRESS.into()));
        let aggregate = IMulticall3::aggregate3Call::abi_decode(tx.input.input().unwrap()).unwrap();
        assert_eq!(aggregate.calls.len(), 2);
        for (call, calldata) in aggregate.calls.iter().zip([
            IMulticall3::getBlockNumberCall {}.abi_encode(),
            IMulticall3::getChainIdCall {}.abi_encode(),
        ]) {
            assert_eq!(call.target, MULTICALL3_ADDRESS);
            assert!(call.allowFailure);
            assert_eq!(call.callData.as_ref(), calldata.as_slice());
        }
    }

    #[tokio::test]
    async fn call_batch_should_preserve_inner_default_block() {
        let asserter = Asserter::new();
        asserter.push_success(&Bytes::from_static(&[0x01]));
        asserter.push_success(&Bytes::from_static(&[0x02]));

        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        historical.get_code_at(Address::ZERO).await.unwrap();

        let batched = CallBatchLayer::new().layer(historical);
        batched.get_code_at(Address::ZERO).await.unwrap();

        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(request.method(), "eth_getCode");
        }
        let before: serde_json::Value =
            serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
        let after: serde_json::Value =
            serde_json::from_str(requests[1].params().unwrap().get()).unwrap();
        assert_eq!(before[1], "0x7b");
        assert_eq!(after[1], before[1]);
    }

    #[tokio::test]
    async fn non_batched_code_preserves_explicit_blocks() {
        let asserter = Asserter::new();
        let code = Bytes::from_static(&[0x01]);
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        let provider = CallBatchLayer::new().layer(historical);

        for block in [BlockId::number(456), BlockId::latest()] {
            asserter.push_success(&code);
            assert_eq!(provider.get_code_at(Address::ZERO).block_id(block).await.unwrap(), code);
            let requests = asserter.take_requests();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].method(), "eth_getCode");
            let params: serde_json::Value =
                serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
            assert_eq!(params, serde_json::json!([Address::ZERO, block]));
        }
    }

    #[tokio::test]
    async fn non_batched_code_defaults_to_latest_without_block_layer() {
        let asserter = Asserter::new();
        let code = Bytes::from_static(&[0x01]);
        asserter.push_success(&code);
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let provider = CallBatchLayer::new().layer(root);

        assert_eq!(provider.get_code_at(Address::ZERO).await.unwrap(), code);
        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), "eth_getCode");
        let params: serde_json::Value =
            serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
        assert_eq!(params, serde_json::json!([Address::ZERO, "latest"]));
    }

    #[tokio::test]
    async fn non_batched_code_preserves_inner_cache() {
        let asserter = Asserter::new();
        let code = Bytes::from_static(&[0x01]);
        asserter.push_success(&code);
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        let cached = CacheLayer::new(10).layer(historical);
        let provider = CallBatchLayer::new().layer(cached);

        assert_eq!(provider.get_code_at(Address::ZERO).await.unwrap(), code);
        assert_eq!(provider.get_code_at(Address::ZERO).await.unwrap(), code);
        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 1, "the second request should be served by the inner cache");
        assert_eq!(requests[0].method(), "eth_getCode");
        let params: serde_json::Value =
            serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
        assert_eq!(params, serde_json::json!([Address::ZERO, "0x7b"]));
    }

    fn assert_rpc_block(asserter: &Asserter, method: &str, block: BlockId) {
        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), method);
        let params: serde_json::Value =
            serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
        assert_eq!(params.as_array().unwrap().last().unwrap(), &serde_json::json!(block));
    }

    // Exercise the inner default and caller-supplied blocks using the real RPC builders.
    macro_rules! assert_non_batched_block {
        ($name:ident, $method:literal, $response:expr, | $provider:ident | $call:expr) => {
            #[tokio::test]
            async fn $name() {
                let asserter = Asserter::new();
                let root: RootProvider =
                    ProviderBuilder::default().connect_mocked_client(asserter.clone());
                let $provider = BlockIdLayer::new(123u64.into()).layer(root);
                asserter.push_success(&$response);
                ($call).await.unwrap();
                assert_rpc_block(&asserter, $method, BlockId::number(123));

                let $provider = CallBatchLayer::new().layer($provider);
                for block in [None, Some(BlockId::number(456)), Some(BlockId::latest())] {
                    asserter.push_success(&$response);
                    match block {
                        Some(block) => ($call).block_id(block).await.unwrap(),
                        None => ($call).await.unwrap(),
                    };
                    assert_rpc_block(&asserter, $method, block.unwrap_or(BlockId::number(123)));
                }
            }
        };
    }

    assert_non_batched_block!(
        forwards_storage_default_block,
        "eth_getStorageAt",
        alloy_primitives::B256::ZERO,
        |provider| provider.get_storage_at(Address::ZERO, U256::ZERO)
    );
    assert_non_batched_block!(
        forwards_transaction_count_default_block,
        "eth_getTransactionCount",
        alloy_primitives::U64::from(7),
        |provider| provider.get_transaction_count(Address::ZERO)
    );
    assert_non_batched_block!(
        forwards_access_list_default_block,
        "eth_createAccessList",
        alloy_rpc_types_eth::AccessListResult::default(),
        |provider| provider.create_access_list(&TransactionRequest::default())
    );
    assert_non_batched_block!(
        forwards_proof_default_block,
        "eth_getProof",
        alloy_rpc_types_eth::EIP1186AccountProofResponse::default(),
        |provider| provider.get_proof(Address::ZERO, vec![])
    );
    assert_non_batched_block!(
        forwards_storage_values_default_block,
        "eth_getStorageValues",
        alloy_rpc_types_eth::StorageValuesResponse::default(),
        |provider| provider
            .get_storage_values(alloy_rpc_types_eth::StorageValuesRequest::default())
    );
    assert_non_batched_block!(
        forwards_account_default_block,
        "eth_getAccount",
        alloy_consensus::TrieAccount::default(),
        |provider| provider.get_account(Address::ZERO)
    );
    assert_non_batched_block!(
        forwards_account_info_default_block,
        "eth_getAccountInfo",
        alloy_rpc_types_eth::AccountInfo::default(),
        |provider| provider.get_account_info(Address::ZERO)
    );
    assert_non_batched_block!(
        forwards_simulate_default_block,
        "eth_simulateV1",
        Vec::<alloy_rpc_types_eth::simulate::SimulatedBlock>::new(),
        |provider| provider.simulate(&alloy_rpc_types_eth::simulate::SimulatePayload::default())
    );

    #[tokio::test]
    async fn forwards_estimate_gas_default_block() {
        let asserter = Asserter::new();
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        asserter.push_success(&alloy_primitives::U64::from(21_000));
        assert_eq!(historical.estimate_gas(TransactionRequest::default()).await.unwrap(), 21_000);
        assert_rpc_block(&asserter, "eth_estimateGas", BlockId::number(123));

        let provider = CallBatchLayer::new().layer(historical);
        for block in [None, Some(BlockId::number(456)), Some(BlockId::latest())] {
            asserter.push_success(&alloy_primitives::U64::from(21_000));
            let call = provider.estimate_gas(TransactionRequest::default());
            let gas = match block {
                Some(block) => call.block(block).await.unwrap(),
                None => call.await.unwrap(),
            };
            assert_eq!(gas, 21_000);
            assert_rpc_block(&asserter, "eth_estimateGas", block.unwrap_or(BlockId::number(123)));
        }
    }

    #[tokio::test]
    async fn forwards_cached_logs() {
        let asserter = Asserter::new();
        asserter.push_success(&Vec::<alloy_rpc_types_eth::Log>::new());
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let provider = CallBatchLayer::new().layer(CacheLayer::new(10).layer(root));
        let filter = alloy_rpc_types_eth::Filter::new().from_block(123).to_block(123);
        for _ in 0..2 {
            assert!(provider.get_logs(&filter).await.unwrap().is_empty());
        }
        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), "eth_getLogs");
    }

    #[tokio::test]
    async fn forwards_cached_block_receipts() {
        let asserter = Asserter::new();
        asserter.push_success(&Some(Vec::<alloy_rpc_types_eth::TransactionReceipt>::new()));
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let provider = CallBatchLayer::new().layer(CacheLayer::new(10).layer(root));
        for _ in 0..2 {
            assert_eq!(
                provider.get_block_receipts(BlockId::number(123)).await.unwrap(),
                Some(vec![])
            );
        }
        assert_rpc_block(&asserter, "eth_getBlockReceipts", BlockId::number(123));
    }

    #[derive(Clone, Debug)]
    struct Recorder {
        inner: RootProvider,
        calls: Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    impl Recorder {
        fn record(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
        }
    }

    #[async_trait::async_trait]
    impl Provider for Recorder {
        fn root(&self) -> &RootProvider {
            self.inner.root()
        }

        fn get_accounts(&self) -> ProviderCall<alloy_rpc_client::NoParams, Vec<Address>> {
            self.record("get_accounts");
            self.inner.get_accounts()
        }

        fn get_gas_price(
            &self,
        ) -> ProviderCall<alloy_rpc_client::NoParams, alloy_primitives::U128, u128> {
            self.record("get_gas_price");
            self.inner.get_gas_price()
        }

        fn get_transaction_by_hash(
            &self,
            hash: alloy_primitives::TxHash,
        ) -> ProviderCall<(alloy_primitives::TxHash,), Option<alloy_rpc_types_eth::Transaction>>
        {
            self.record("get_transaction_by_hash");
            self.inner.get_transaction_by_hash(hash)
        }

        async fn get_header(
            &self,
            block: BlockId,
        ) -> TransportResult<Option<alloy_rpc_types_eth::Header>> {
            self.record("get_header");
            self.inner.get_header(block).await
        }

        fn call_many<'req>(
            &self,
            bundles: &'req [alloy_rpc_types_eth::Bundle],
        ) -> crate::EthCallMany<'req, Ethereum, Vec<Vec<alloy_rpc_types_eth::EthCallResponse>>>
        {
            self.record("call_many");
            self.inner.call_many(bundles)
        }

        fn call(&self, tx: TransactionRequest) -> crate::EthCall<Ethereum, Bytes> {
            self.record("call");
            self.inner.call(tx)
        }

        async fn send_transaction(
            &self,
            _tx: TransactionRequest,
        ) -> TransportResult<crate::PendingTransactionBuilder<Ethereum>> {
            self.record("send_transaction");
            Err(TransportErrorKind::custom_str("recorder"))
        }

        async fn get_filter_changes<R: alloy_json_rpc::RpcRecv>(
            &self,
            _id: U256,
        ) -> TransportResult<Vec<R>> {
            self.record("get_filter_changes");
            Ok(vec![])
        }

        async fn raw_request<Params, R>(
            &self,
            _method: std::borrow::Cow<'static, str>,
            _params: Params,
        ) -> TransportResult<R>
        where
            Params: alloy_json_rpc::RpcSend,
            R: alloy_json_rpc::RpcRecv,
        {
            self.record("raw_request");
            Err(TransportErrorKind::custom_str("recorder"))
        }

        #[cfg(feature = "pubsub")]
        fn subscribe<Params, R>(&self, params: Params) -> crate::GetSubscription<Params, R>
        where
            Params: alloy_json_rpc::RpcSend,
            R: alloy_json_rpc::RpcRecv,
        {
            self.record("subscribe");
            self.inner.subscribe(params)
        }

        #[cfg(feature = "pubsub")]
        fn subscribe_to<R>(
            &self,
            method: &'static str,
        ) -> crate::GetSubscription<alloy_rpc_client::NoParams, R>
        where
            R: alloy_json_rpc::RpcRecv,
        {
            self.record("subscribe_to");
            self.inner.subscribe_to(method)
        }
    }

    #[tokio::test]
    async fn forwards_non_batched_inner_overrides() {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(Asserter::new());
        let provider = CallBatchLayer::new().layer(Recorder { inner: root, calls: calls.clone() });
        let _ = provider.get_accounts().await;
        let _ = provider.get_gas_price().await;
        let _ = provider.get_transaction_by_hash(alloy_primitives::B256::ZERO).await;
        let _ = provider.call_many(&[]).await;
        let _ = provider.send_transaction(TransactionRequest::default()).await;
        let _ = provider.get_filter_changes::<alloy_primitives::B256>(U256::ZERO).await;
        let _ = provider.raw_request::<_, serde_json::Value>("custom".into(), ()).await;
        assert_eq!(
            *calls.lock().unwrap(),
            [
                "get_accounts",
                "get_gas_price",
                "get_transaction_by_hash",
                "call_many",
                "send_transaction",
                "get_filter_changes",
                "raw_request",
            ]
        );
    }

    #[tokio::test]
    #[cfg(feature = "pubsub")]
    async fn forwards_generic_subscription_overrides() {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(Asserter::new());
        let provider = CallBatchLayer::new().layer(Recorder { inner: root, calls: calls.clone() });
        drop(provider.subscribe::<_, alloy_primitives::B256>((
            alloy_rpc_types_eth::pubsub::SubscriptionKind::NewHeads,
        )));
        drop(provider.subscribe_to::<alloy_primitives::B256>("custom_subscription"));
        assert_eq!(*calls.lock().unwrap(), ["subscribe", "subscribe_to"]);
    }

    #[tokio::test]
    async fn forwards_with_any_network_and_type_erasure() {
        let asserter = Asserter::new();
        asserter.push_success(&alloy_primitives::U64::from(21_000));
        let root: RootProvider<alloy_network::AnyNetwork> =
            ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        let provider = CallBatchLayer::new().layer(historical).erased();
        assert_eq!(provider.estimate_gas(Default::default()).await.unwrap(), 21_000);
        assert_rpc_block(&asserter, "eth_estimateGas", BlockId::number(123));
    }

    #[tokio::test]
    async fn block_number_by_id_preserves_inner_header_dispatch() {
        let asserter = Asserter::new();
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let provider = CallBatchLayer::new().layer(Recorder { inner: root, calls: calls.clone() });

        assert_eq!(provider.get_block_number_by_id(BlockId::number(456)).await.unwrap(), Some(456));
        assert!(asserter.requests().is_empty());
        assert!(calls.lock().unwrap().is_empty());

        let mut header: alloy_rpc_types_eth::Header = Default::default();
        header.inner.number = 77;
        for block in [BlockId::hash(B256::repeat_byte(1)), BlockId::safe()] {
            asserter.push_success(&Some(header.clone()));
            assert_eq!(provider.get_block_number_by_id(block).await.unwrap(), Some(77));
            let requests = asserter.take_requests();
            assert_eq!(requests.len(), 1);
            let params: serde_json::Value =
                serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
            match block {
                BlockId::Hash(hash) => {
                    assert_eq!(requests[0].method(), "eth_getHeaderByHash");
                    assert_eq!(params, serde_json::json!([hash.block_hash]));
                }
                BlockId::Number(number) => {
                    assert_eq!(requests[0].method(), "eth_getHeaderByNumber");
                    assert_eq!(params, serde_json::json!([number]));
                }
            }
        }
        asserter.push_success(&Option::<alloy_rpc_types_eth::Header>::None);
        assert_eq!(provider.get_block_number_by_id(BlockId::hash(B256::ZERO)).await.unwrap(), None);
        let requests = asserter.take_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), "eth_getHeaderByHash");
        assert_eq!(*calls.lock().unwrap(), ["get_header", "get_header", "get_header"]);
    }

    #[tokio::test]
    async fn non_batchable_calls_keep_existing_root_dispatch() {
        let asserter = Asserter::new();
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let recorder = Recorder { inner: root, calls: calls.clone() };
        let historical = BlockIdLayer::new(123u64.into()).layer(recorder);
        let tx = TransactionRequest::default().with_to(COUNTER_ADDRESS).with_gas_limit(21_000);
        asserter.push_success(&Bytes::new());
        historical.call(tx.clone()).await.unwrap();
        assert_rpc_block(&asserter, "eth_call", BlockId::number(123));
        assert_eq!(*calls.lock().unwrap(), ["call"]);
        calls.lock().unwrap().clear();

        let provider = CallBatchLayer::new().layer(historical);
        for block in [None, Some(BlockId::number(456))] {
            asserter.push_success(&Bytes::new());
            let call = provider.call(tx.clone());
            match block {
                Some(block) => call.block(block).await.unwrap(),
                None => call.await.unwrap(),
            };
            if let Some(block) = block {
                assert_rpc_block(&asserter, "eth_call", block);
            } else {
                let requests = asserter.take_requests();
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0].method(), "eth_call");
                let params: serde_json::Value =
                    serde_json::from_str(requests[0].params().unwrap().get()).unwrap();
                assert_eq!(params, serde_json::json!([tx]));
            }
        }
        // This documents an existing limitation, not the ideal intended behavior.
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn batchable_calls_keep_latest_with_inner_default() {
        let asserter = Asserter::new();
        let root: RootProvider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
        let historical = BlockIdLayer::new(123u64.into()).layer(root);
        let provider = CallBatchLayer::new().layer(historical);
        let tx =
            TransactionRequest::default().with_to(COUNTER_ADDRESS).with_input(hex!("0x8381f58a"));
        let code = Bytes::from_static(&[0x01]);

        // The backend's single-request path still overrides the inner historical default.
        asserter.push_success(&code);
        assert_eq!(provider.call(tx.clone()).await.unwrap(), code);
        assert_rpc_block(&asserter, "eth_call", BlockId::latest());

        push_m3_success(&asserter, &[(true, code.to_vec()), (true, U256::from(7).abi_encode())]);
        let (call, balance) =
            tokio::join!(provider.call(tx), provider.get_balance(COUNTER_ADDRESS));
        assert_eq!(call.unwrap(), code);
        assert_eq!(balance.unwrap(), U256::from(7));
        assert_rpc_block(&asserter, "eth_call", BlockId::latest());
    }

    #[tokio::test]
    async fn basic_mocked() {
        let asserter = Asserter::new();
        let provider =
            ProviderBuilder::new().with_call_batching().connect_mocked_client(asserter.clone());
        push_m3_success(
            &asserter,
            &[
                (true, 1.abi_encode()),  // IMulticall3::getBlockNumberCall
                (true, 2.abi_encode()),  // IMulticall3::getChainIdCall
                (false, 3.abi_encode()), // IMulticall3::getBlockNumberCall
                (false, 4.abi_encode()), // IMulticall3::getChainIdCall
            ],
        );
        let (block_number_ok, chain_id_ok, block_number_err, chain_id_err) = tokio::join!(
            provider.get_block_number(),
            provider.get_chain_id(),
            provider.get_block_number(),
            provider.get_chain_id(),
        );
        assert_eq!(block_number_ok.unwrap(), 1);
        assert_eq!(chain_id_ok.unwrap(), 2);
        assert!(block_number_err.unwrap_err().to_string().contains("reverted"));
        assert!(chain_id_err.unwrap_err().to_string().contains("reverted"));
        assert!(asserter.read_q().is_empty(), "only 1 request should've been made");
    }

    #[tokio::test]
    async fn batch_response_len_mismatch_errors_all_callers() {
        let asserter = Asserter::new();
        let provider =
            ProviderBuilder::new().with_call_batching().connect_mocked_client(asserter.clone());

        push_m3_success(&asserter, &[(true, 1.abi_encode())]);

        let (block_number, chain_id) =
            tokio::join!(provider.get_block_number(), provider.get_chain_id());

        let block_number_err = block_number.unwrap_err().to_string();
        let chain_id_err = chain_id.unwrap_err().to_string();
        assert!(block_number_err.contains("response count mismatch"), "{block_number_err}");
        assert!(chain_id_err.contains("response count mismatch"), "{chain_id_err}");
        assert!(asserter.read_q().is_empty(), "only 1 request should've been made");
    }

    #[tokio::test]
    async fn batch_preserves_rpc_error_payloads() {
        let asserter = Asserter::new();
        let provider =
            ProviderBuilder::new().with_call_batching().connect_mocked_client(asserter.clone());
        let payload =
            serde_json::from_str(r#"{"code":-32005,"message":"rate limited","data":{"retry":5}}"#)
                .unwrap();
        asserter.push_failure(payload);

        let (block_number, chain_id) =
            tokio::join!(provider.get_block_number(), provider.get_chain_id());
        for error in [block_number.unwrap_err(), chain_id.unwrap_err()] {
            let payload = error.as_error_resp().unwrap();
            assert_eq!(payload.code, -32005);
            assert_eq!(payload.message, "rate limited");
            assert_eq!(payload.data.as_ref().unwrap().get(), r#"{"retry":5}"#);
        }
        assert_eq!(asserter.requests().len(), 1);
    }

    #[test]
    fn should_not_batch_calls_with_block_overrides() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let inner = CallBatchProviderInner::<Ethereum> { tx };
        let params =
            crate::EthCallParams::new(TransactionRequest::default().with_to(COUNTER_ADDRESS))
                .with_block_overrides(BlockOverrides::default().with_number(U256::from(1)));

        assert!(!inner.should_batch_call(&params));
    }

    #[test]
    fn should_not_batch_calls_with_unrepresentable_fields() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let inner = CallBatchProviderInner::<Ethereum> { tx };

        // Only `to`/`data`/`input` survive the conversion into a `Call3`.
        let ok = crate::EthCallParams::new(
            TransactionRequest::default().with_to(COUNTER_ADDRESS).with_input(hex!("0x8381f58a")),
        );
        assert!(inner.should_batch_call(&ok));

        // Anything else has to go out as a regular `eth_call`.
        let extra = crate::EthCallParams::new(
            TransactionRequest::default().with_to(COUNTER_ADDRESS).with_gas_limit(1),
        );
        assert!(!inner.should_batch_call(&extra));
    }

    #[test]
    fn multicall_compatibility_is_fail_closed() {
        // A `Call3` carries a target and calldata, and nothing else.
        assert!(is_multicall_compatible(&serde_json::json!({})));
        assert!(is_multicall_compatible(&serde_json::json!({ "to": COUNTER_ADDRESS })));
        assert!(is_multicall_compatible(
            &serde_json::json!({ "to": COUNTER_ADDRESS, "data": "0x8381f58a" })
        ));

        // Any other field would be dropped when building the multicall.
        assert!(!is_multicall_compatible(
            &serde_json::json!({ "to": COUNTER_ADDRESS, "gas": "0x1" })
        ));

        // Shapes we cannot inspect at all must never be batched.
        assert!(!is_multicall_compatible(&serde_json::json!([])));
        assert!(!is_multicall_compatible(&serde_json::json!("0x")));
        assert!(!is_multicall_compatible(&serde_json::Value::Null));
    }

    #[tokio::test]
    #[cfg(feature = "anvil-api")]
    async fn basic() {
        use crate::ext::AnvilApi;
        let provider = ProviderBuilder::new().with_call_batching().connect_anvil();
        provider.anvil_set_code(COUNTER_ADDRESS, COUNTER_DEPLOYED_CODE.into()).await.unwrap();
        provider.anvil_set_balance(COUNTER_ADDRESS, U256::from(123)).await.unwrap();

        let do_calls = || async {
            tokio::join!(
                provider.call(
                    TransactionRequest::default()
                        .with_to(COUNTER_ADDRESS)
                        .with_input(hex!("0x8381f58a")) // number()
                ),
                provider.call(
                    TransactionRequest::default()
                        .with_to(MULTICALL3_ADDRESS)
                        .with_input(IMulticall3::getBlockNumberCall {}.abi_encode())
                ),
                provider.get_block_number(),
                provider.get_chain_id(),
                provider.get_balance(COUNTER_ADDRESS),
            )
        };

        // Multicall3 has not yet been deployed.
        let (a, b, c, d, e) = do_calls().await;
        assert!(a.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(b.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(c.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(d.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(e.unwrap_err().to_string().contains("Multicall3 not deployed"));

        provider.anvil_set_code(MULTICALL3_ADDRESS, MULTICALL3_DEPLOYED_CODE.into()).await.unwrap();
        let head = provider.get_block_number().await.unwrap();

        let (counter, block_number_raw, block_number, chain_id, balance) = do_calls().await;
        assert_eq!(counter.unwrap(), 0u64.abi_encode());
        assert_eq!(block_number_raw.unwrap(), head.abi_encode());
        assert_eq!(block_number.unwrap(), head);
        assert_eq!(chain_id.unwrap(), alloy_chains::NamedChain::AnvilHardhat as u64);
        assert_eq!(balance.unwrap(), U256::from(123));
    }

    #[tokio::test]
    #[cfg(feature = "anvil-api")]
    async fn ignores_inner_default_block() {
        use crate::ext::AnvilApi;
        let provider = ProviderBuilder::new()
            .with_call_batching()
            .with_default_block(1.into())
            .connect_anvil();
        provider.anvil_set_code(MULTICALL3_ADDRESS, MULTICALL3_DEPLOYED_CODE.into()).await.unwrap();
        provider.anvil_set_balance(COUNTER_ADDRESS, U256::from(100)).await.unwrap();
        provider.anvil_mine(Some(2), None).await.unwrap();
        provider.anvil_set_balance(COUNTER_ADDRESS, U256::from(200)).await.unwrap();

        // A lone request takes the single-call fallback, a joined one goes through Multicall3.
        let single = provider.get_balance(COUNTER_ADDRESS).await.unwrap();
        let (batched, _) =
            tokio::join!(provider.get_balance(COUNTER_ADDRESS), provider.get_chain_id());
        assert_eq!(single, U256::from(200));
        assert_eq!(batched.unwrap(), U256::from(200));
    }
}
