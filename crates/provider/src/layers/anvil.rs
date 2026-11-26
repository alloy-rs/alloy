use crate::{PendingTransactionBuilder, Provider, ProviderLayer, RootProvider, SendableTx};
use alloy_json_rpc::RpcRecv;
use alloy_network::{Ethereum, Network};
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::Bytes;
use alloy_rpc_types_eth::{erc4337::TransactionConditional, FillTransaction};
use alloy_transport::TransportResult;
use reqwest::Url;
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

/// A layer that wraps an [`Anvil`] config.
///
/// The config will be used to spawn an [`AnvilInstance`] when the layer is applied, or when the
/// user requests any information about the anvil node (e.g. via the [`AnvilLayer::ws_endpoint_url`]
/// method).
#[derive(Debug, Clone, Default)]
pub struct AnvilLayer {
    anvil: Anvil,
    instance: OnceLock<Arc<AnvilInstance>>,
}

impl AnvilLayer {
    /// Starts the anvil instance, or gets a reference to the existing instance.
    pub fn instance(&self) -> &Arc<AnvilInstance> {
        self.instance.get_or_init(|| Arc::new(self.anvil.clone().spawn()))
    }

    /// Get the instance http endpoint.
    #[doc(alias = "http_endpoint_url")]
    pub fn endpoint_url(&self) -> Url {
        self.instance().endpoint_url()
    }

    /// Get the instance ws endpoint.
    pub fn ws_endpoint_url(&self) -> Url {
        self.instance().ws_endpoint_url()
    }
}

impl From<Anvil> for AnvilLayer {
    fn from(anvil: Anvil) -> Self {
        Self { anvil, instance: OnceLock::new() }
    }
}

impl<P: Provider<N>, N: Network> ProviderLayer<P, N> for AnvilLayer {
    type Provider = AnvilProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        let anvil = self.instance();
        AnvilProvider::new(inner, anvil.clone())
    }
}

/// A provider that wraps an [`AnvilInstance`], preventing the instance from
/// being dropped while the provider is in use.
#[derive(Clone, Debug)]
pub struct AnvilProvider<P, N = Ethereum> {
    inner: P,
    anvil: Arc<AnvilInstance>,
    _marker: PhantomData<N>,
}

impl<P: Provider<N>, N: Network> AnvilProvider<P, N> {
    /// Creates a new `AnvilProvider` with the given inner provider and anvil
    /// instance.
    #[expect(clippy::missing_const_for_fn)]
    pub fn new(inner: P, anvil: Arc<AnvilInstance>) -> Self {
        Self { inner, anvil, _marker: PhantomData }
    }

    /// Expose inner anvil instance.
    pub const fn anvil(&self) -> &Arc<AnvilInstance> {
        &self.anvil
    }
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<P: Provider<N>, N: Network> Provider<N> for AnvilProvider<P, N> {
    #[inline(always)]
    fn root(&self) -> &RootProvider<N> {
        self.inner.root()
    }

    async fn send_raw_transaction(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_raw_transaction(encoded_tx).await
    }

    async fn send_raw_transaction_sync(
        &self,
        encoded_tx: &[u8],
    ) -> TransportResult<N::ReceiptResponse> {
        self.inner.send_raw_transaction_sync(encoded_tx).await
    }

    async fn send_raw_transaction_conditional(
        &self,
        encoded_tx: &[u8],
        conditional: TransactionConditional,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_raw_transaction_conditional(encoded_tx, conditional).await
    }

    async fn send_transaction_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<PendingTransactionBuilder<N>> {
        self.inner.send_transaction_internal(tx).await
    }

    async fn send_transaction_sync_internal(
        &self,
        tx: SendableTx<N>,
    ) -> TransportResult<N::ReceiptResponse> {
        self.inner.send_transaction_sync_internal(tx).await
    }

    async fn sign_transaction(&self, tx: N::TransactionRequest) -> TransportResult<Bytes> {
        self.inner.sign_transaction(tx).await
    }

    async fn fill_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<FillTransaction<N::TxEnvelope>>
    where
        N::TxEnvelope: RpcRecv,
    {
        self.inner.fill_transaction(tx).await
    }

    async fn fill_and_sign_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> TransportResult<N::TxEnvelope> {
        self.inner.fill_and_sign_transaction(tx).await
    }

    fn transaction_request(&self) -> N::TransactionRequest {
        self.inner.transaction_request()
    }
}
