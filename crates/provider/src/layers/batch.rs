use crate::{
    bindings::IMulticall3, Caller, Provider, ProviderLayer, RootProvider, MULTICALL3_ADDRESS,
};
use alloy_eips::BlockId;
use alloy_json_rpc::RpcRecv;
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes};
use alloy_rpc_client::WeakClient;
use alloy_sol_types::{SolCall, SolType, SolValue};
use alloy_transport::{utils::Spawnable, TransportErrorKind, TransportResult};
use std::{fmt, marker::PhantomData, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;

/// A layer that batches multiple requests into a single request.
#[derive(Debug)]
pub struct BatchLayer {
    m3a: Address,
    wait: Duration,
}

impl BatchLayer {
    /// Create a new `BatchLayer` with a default wait of 1ms.
    pub fn new() -> Self {
        Self { m3a: MULTICALL3_ADDRESS, wait: Duration::from_millis(1) }
    }

    /// Set the amount of time to wait before sending the batch.
    pub fn wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Set the multicall3 address.
    pub fn multicall3_address(mut self, m3a: Address) -> Self {
        self.m3a = m3a;
        self
    }
}

impl<P, N> ProviderLayer<P, N> for BatchLayer
where
    P: Provider<N> + 'static,
    N: Network,
{
    type Provider = BatchProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        BatchProvider::new(inner, self)
    }
}

type ErasedResult = TransportResult<Bytes>;

struct BatchProviderMessage {
    call: IMulticall3::Call3,
    tx: oneshot::Sender<ErasedResult>,
}
#[derive(Debug)]
enum BatchProviderMessageKind<N: Network = Ethereum> {
    Call(Option<BlockId>, N::TransactionRequest),
    BlockNumber,
    ChainId,
}

impl BatchProviderMessage {
    fn new<N: Network>(
        kind: BatchProviderMessageKind<N>,
        m3a: Address,
    ) -> (Self, oneshot::Receiver<ErasedResult>) {
        let (tx, rx) = oneshot::channel();
        (Self { call: kind.into_call3(m3a), tx }, rx)
    }
}

impl<N: Network> BatchProviderMessageKind<N> {
    fn into_call3(self, m3a: Address) -> IMulticall3::Call3 {
        let m3a_call = |data: Vec<u8>| IMulticall3::Call3 {
            target: m3a,
            allowFailure: true,
            callData: data.into(),
        };
        match self {
            Self::Call(_, tx) => IMulticall3::Call3 {
                target: tx.to().unwrap_or_default(),
                allowFailure: true,
                callData: tx.input().cloned().unwrap_or_default(),
            },
            Self::BlockNumber => m3a_call(IMulticall3::getBlockNumberCall {}.abi_encode()),
            Self::ChainId => m3a_call(IMulticall3::getChainIdCall {}.abi_encode()),
        }
    }

    fn block(&self) -> Option<BlockId> {
        if let Self::Call(block, _) = self {
            *block
        } else {
            None
        }
    }
}

/// A provider that batches multiple requests into a single request.
///
/// See [`BatchLayer`] for more information.
pub struct BatchProvider<P, N: Network = Ethereum> {
    provider: Arc<P>,
    inner: BatchProviderInner,
    _pd: PhantomData<N>,
}

impl<P: fmt::Debug, N: Network> fmt::Debug for BatchProvider<P, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BatchProvider(")?;
        self.provider.fmt(f)?;
        f.write_str(")")
    }
}

impl<P: Provider<N> + 'static, N: Network> BatchProvider<P, N> {
    fn new(inner: P, layer: &BatchLayer) -> Self {
        let inner = Arc::new(inner);
        let tx = BatchProviderBackend::spawn(inner.clone(), layer);
        Self { provider: inner, inner: BatchProviderInner { tx, m3a: layer.m3a }, _pd: PhantomData }
    }
}

#[derive(Clone)]
struct BatchProviderInner {
    tx: mpsc::UnboundedSender<BatchProviderMessage>,
    m3a: Address,
}

impl BatchProviderInner {
    async fn schedule<N: Network, T: SolValue>(
        self,
        msg: BatchProviderMessageKind<N>,
    ) -> TransportResult<T>
    where
        T: From<<T::SolType as SolType>::RustType>,
    {
        let (msg, rx) = BatchProviderMessage::new(msg, self.m3a);
        self.tx.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;
        let result_bytes = rx.await.map_err(|_| TransportErrorKind::backend_gone())??;
        let result = T::abi_decode(&result_bytes, false).map_err(TransportErrorKind::custom)?;
        Ok(result)
    }
}

struct BatchProviderBackend<P, N: Network = Ethereum> {
    inner: Arc<P>,
    m3a: Address,
    wait: Duration,
    rx: mpsc::UnboundedReceiver<BatchProviderMessage>,
    pending: Vec<BatchProviderMessage>,
    _pd: PhantomData<N>,
}

impl<P: Provider<N> + 'static, N: Network> BatchProviderBackend<P, N> {
    fn spawn(inner: Arc<P>, layer: &BatchLayer) -> mpsc::UnboundedSender<BatchProviderMessage> {
        let BatchLayer { m3a, wait } = *layer;
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

            // Handle messages within the wait.
            debug_assert!(!self.pending.is_empty());
            'inner: loop {
                sleep(self.wait).await;
                match self.rx.try_recv() {
                    Ok(msg) => self.process_msg(msg),
                    Err(mpsc::error::TryRecvError::Empty) => {
                        // No more messages, send the batch.
                        self.send_batch().await;
                        break 'inner;
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => break 'outer,
                }
            }
        }
    }

    fn process_msg(&mut self, msg: BatchProviderMessage) {
        self.pending.push(msg);
    }

    async fn send_batch(&mut self) {
        match self.send_batch_inner().await {
            Ok(results) => {
                let pending = std::mem::take(&mut self.pending);
                debug_assert_eq!(results.len(), pending.len());
                for (result, msg) in results.into_iter().zip(pending) {
                    // TODO: handle result.success ?
                    let _ = msg.tx.send(Ok(result.returnData));
                }
            }
            Err(e) => {
                for msg in std::mem::take(&mut self.pending) {
                    let _ = msg.tx.send(Err(TransportErrorKind::custom_str(&e.to_string())));
                }
            }
        }
    }

    async fn send_batch_inner(&mut self) -> TransportResult<Vec<IMulticall3::Result>> {
        debug_assert!(!self.pending.is_empty());
        let tx = N::TransactionRequest::default().with_to(self.m3a).with_input(self.make_payload());
        let bytes = self.inner.call(tx).await?;
        let ret = IMulticall3::aggregate3Call::abi_decode_returns(&bytes, false)
            .map_err(TransportErrorKind::custom)?;
        Ok(ret.returnData)
    }

    fn make_payload(&self) -> Vec<u8> {
        IMulticall3::aggregate3Call {
            calls: self.pending.iter().map(|msg| msg.call.clone()).collect(),
        }
        .abi_encode()
    }
}

impl<P: Provider<N>, N: Network> Provider<N> for BatchProvider<P, N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }

    fn call(&self, tx: <N as Network>::TransactionRequest) -> crate::EthCall<N, Bytes> {
        crate::EthCall::call(BatchProviderCaller::new(self), tx)
    }

    fn get_block_number(
        &self,
    ) -> crate::ProviderCall<
        alloy_rpc_client::NoParams,
        alloy_primitives::U64,
        alloy_primitives::BlockNumber,
    > {
        crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule::<N, u64>(BatchProviderMessageKind::BlockNumber),
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
            self.inner.clone().schedule::<N, u64>(BatchProviderMessageKind::ChainId),
        ))
    }

    // TODO: override rest with self.provider
}

struct BatchProviderCaller {
    inner: BatchProviderInner,
    weak: WeakClient,
}

impl BatchProviderCaller {
    fn new<P: Provider<N>, N: Network>(provider: &BatchProvider<P, N>) -> Self {
        Self { inner: provider.inner.clone(), weak: provider.provider.weak_client() }
    }
}

impl<N: Network, Resp> Caller<N, Resp> for BatchProviderCaller
where
    Resp: RpcRecv + SolValue,
    Resp: From<<Resp::SolType as SolType>::RustType>,
{
    fn call(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Resp>> {
        // TODO: overrides?
        Ok(crate::ProviderCall::BoxedFuture(Box::pin(self.inner.clone().schedule::<N, Resp>(
            BatchProviderMessageKind::Call(params.block, params.into_data()),
        ))))
    }

    fn estimate_gas(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Resp>> {
        Caller::<N, Resp>::estimate_gas(&self.weak, params)
    }

    fn call_many(
        &self,
        params: crate::EthCallManyParams<'_>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallManyParams<'static>, Resp>> {
        Caller::<N, Resp>::call_many(&self.weak, params)
    }
}

#[cfg(any())] // TODO
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::MockProvider;

    #[test]
    fn basic() {
        let (provider, asserter) = crate::ProviderBuilder::mocked();
        MockProvider
    }
}
