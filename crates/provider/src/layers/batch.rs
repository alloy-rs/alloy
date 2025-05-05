use crate::{
    bindings::IMulticall3, Caller, Provider, ProviderCall, ProviderLayer, RootProvider,
    MULTICALL3_ADDRESS,
};
use alloy_eips::BlockId;
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_client::WeakClient;
use alloy_sol_types::{SolCall, SolType, SolValue};
use alloy_transport::{utils::Spawnable, TransportErrorKind, TransportResult};
use std::{fmt, future::IntoFuture, marker::PhantomData, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

#[cfg(not(target_family = "wasm"))]
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
}

impl Default for CallBatchLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum CallResult {
    Multicall(IMulticall3::Result),
    Single(Bytes),
}

impl CallBatchLayer {
    /// Create a new `CallBatchLayer` with a default wait of 1ms.
    pub fn new() -> Self {
        Self { m3a: MULTICALL3_ADDRESS, wait: DEFAULT_WAIT }
    }

    /// Set the amount of time to wait before sending the batch.
    ///
    /// This is the amount of time to wait after the first request is received before sending all
    /// the requests received in that time period.
    ///
    /// This means that every request has a maximum delay of `wait` before being sent.
    ///
    /// The default is 1ms.
    pub fn wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Set the multicall3 address.
    ///
    /// The default is [`MULTICALL3_ADDRESS`].
    pub fn multicall3_address(mut self, m3a: Address) -> Self {
        self.m3a = m3a;
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

type CallBatchMsgTx = TransportResult<CallResult>;

struct CallBatchMsg<N: Network> {
    kind: CallBatchMsgKind<N>,
    tx: oneshot::Sender<CallBatchMsgTx>,
}

impl<N: Network> Clone for CallBatchMsgKind<N>
where
    N::TransactionRequest: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Call(tx) => Self::Call(tx.clone()),
            Self::BlockNumber => Self::BlockNumber,
            Self::ChainId => Self::ChainId,
            Self::Balance(addr) => Self::Balance(*addr),
        }
    }
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
    fn new(
        kind: CallBatchMsgKind<N>,
        //m3a: Address,
    ) -> (Self, oneshot::Receiver<CallBatchMsgTx>) {
        let (tx, rx) = oneshot::channel();
        (Self { kind, tx }, rx)
    }
}

impl<N: Network> CallBatchMsgKind<N> {
    fn into_call3(self, m3a: Address) -> IMulticall3::Call3 {
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
            Self::BlockNumber => m3a_call(IMulticall3::getBlockNumberCall {}.abi_encode()),
            Self::ChainId => m3a_call(IMulticall3::getChainIdCall {}.abi_encode()),
            Self::Balance(addr) => m3a_call(IMulticall3::getEthBalanceCall { addr }.abi_encode()),
        }
    }
}

/// A provider that batches multiple requests into a single request.
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
        Self {
            provider: inner,
            inner: CallBatchProviderInner { tx, m3a: layer.m3a },
            _pd: PhantomData,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct CallBatchProviderInner<N: Network> {
    tx: mpsc::UnboundedSender<CallBatchMsg<N>>,
    m3a: Address,
}

impl<N: Network> CallBatchProviderInner<N> {
    /// We only want to perform a scheduled multicall if:
    /// - The request has no block ID or state overrides,
    /// - The request has a target address,
    /// - The request has no other properties (`nonce`, `gas`, etc cannot be sent with a multicall).
    ///
    /// Ref: <https://github.com/wevm/viem/blob/ba8319f71503af8033fd3c77cfb64c7eb235c6a9/src/actions/public/call.ts#L295>
    fn should_batch_call(&self, params: &crate::EthCallParams<N>) -> bool {
        // TODO: block ID is not yet implemented
        if params.block().is_some_and(|block| block != BlockId::latest()) {
            return false;
        }
        if params.overrides.as_ref().is_some_and(|overrides| !overrides.is_empty()) {
            return false;
        }
        let tx = params.data();
        if tx.to().is_none() {
            return false;
        }
        if let Ok(serde_json::Value::Object(obj)) = serde_json::to_value(tx) {
            if obj.keys().any(|k| !matches!(k.as_str(), "to" | "data" | "input")) {
                return false;
            }
        }
        true
    }

    async fn schedule(self, msg: CallBatchMsgKind<N>) -> TransportResult<Bytes> {
        let (msg, rx) = CallBatchMsg::new(msg);
        self.tx.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;

        let result = rx.await.map_err(|_| TransportErrorKind::backend_gone())??;

        match result {
            CallResult::Multicall(IMulticall3::Result { success, returnData }) => {
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
            CallResult::Single(_) => {
                Err(TransportErrorKind::custom_str("expected multicall result"))
            }
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
    rx: mpsc::UnboundedReceiver<CallBatchMsg<N>>,
    pending: Vec<CallBatchMsg<N>>,
    _pd: PhantomData<N>,
}

impl<P: Provider<N> + 'static, N: Network> CallBatchBackend<P, N> {
    fn spawn(inner: Arc<P>, layer: &CallBatchLayer) -> mpsc::UnboundedSender<CallBatchMsg<N>> {
        let CallBatchLayer { m3a, wait } = *layer;
        let (tx, rx) = mpsc::unbounded_channel();
        let this = Self { inner, m3a, wait, rx, pending: Vec::new(), _pd: PhantomData };
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
        let pending = std::mem::take(&mut self.pending);

        if pending.len() == 1 {
            let msg = pending.into_iter().next().unwrap();

            let result: Result<CallResult, _> = match msg.kind {
                CallBatchMsgKind::Call(tx) => {
                    let res = self.inner.call(tx).await.unwrap();
                    Ok(CallResult::Single(res))
                }
                CallBatchMsgKind::BlockNumber => {
                    let result = self.inner.get_block_number().into_future().await.unwrap();
                    Ok(CallResult::Single(Bytes::from(result.to_be_bytes())))
                }
                CallBatchMsgKind::ChainId => {
                    let result = self.inner.get_chain_id().into_future().await.unwrap();
                    Ok(CallResult::Single(Bytes::from(result.to_be_bytes())))
                }
                CallBatchMsgKind::Balance(addr) => {
                    let result = self.inner.get_balance(addr).into_future().await.unwrap();
                    Ok(CallResult::Single(Bytes::from(result.to_be_bytes::<32>())))
                }
            };

            let _ = msg.tx.send(result);
            return;
        }

        let result = self.send_batch_inner(&pending).await;
        match result {
            Ok(results) => {
                debug_assert_eq!(results.len(), pending.len());
                for (result, msg) in results.into_iter().zip(pending) {
                    let _ = msg.tx.send(Ok(CallResult::Multicall(result)));
                }
            }
            Err(e) => {
                for msg in pending {
                    let _ = msg.tx.send(Err(TransportErrorKind::custom_str(&e.to_string())));
                }
            }
        }
    }

    async fn send_batch_inner(
        &self,
        pending: &[CallBatchMsg<N>],
    ) -> TransportResult<Vec<IMulticall3::Result>> {
        let call3s: Vec<_> =
            pending.iter().map(|msg| msg.kind.clone().into_call3(self.m3a)).collect();

        let tx = N::TransactionRequest::default()
            .with_to(self.m3a)
            .with_input(IMulticall3::aggregate3Call { calls: call3s }.abi_encode());

        let bytes = self.inner.call(tx).await?;
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

impl<P: Provider<N> + 'static, N: Network> Provider<N> for CallBatchProvider<P, N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }

    fn call(&self, tx: <N as Network>::TransactionRequest) -> crate::EthCall<N, Bytes> {
        crate::EthCall::call(CallBatchCaller::new(self), tx)
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
