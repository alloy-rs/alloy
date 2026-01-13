use crate::{
    fillers::{
        BlobGasEstimator, BlobGasFiller, CachedNonceManager, ChainIdFiller, FillerControlFlow,
        GasFiller, JoinFill, NonceFiller, NonceManager, RecommendedFillers, SimpleNonceManager,
        TxFiller, WalletFiller,
    },
    layers::{CallBatchLayer, ChainLayer},
    provider::SendableTx,
    Provider, RootProvider,
};
use alloy_chains::NamedChain;
use alloy_network::{Ethereum, IntoWallet, Network};
use alloy_primitives::ChainId;
use alloy_rpc_client::{ClientBuilder, ConnectionConfig, RpcClient};
use alloy_transport::{TransportConnect, TransportError, TransportResult};
use std::marker::PhantomData;

/// A layering abstraction in the vein of [`tower::Layer`]
///
/// [`tower::Layer`]: https://docs.rs/tower/latest/tower/trait.Layer.html
pub trait ProviderLayer<P: Provider<N>, N: Network = Ethereum> {
    /// The provider constructed by this layer.
    type Provider: Provider<N>;

    /// Wrap the given provider in the layer's provider.
    fn layer(&self, inner: P) -> Self::Provider;
}

/// An identity layer that does nothing.
#[derive(Clone, Copy, Debug)]
pub struct Identity;

impl<N> TxFiller<N> for Identity
where
    N: Network,
{
    type Fillable = ();

    fn status(&self, _tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        FillerControlFlow::Finished
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        _provider: &P,
        _tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable> {
        Ok(())
    }

    async fn fill(
        &self,
        _to_fill: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        Ok(tx)
    }
}

impl<P, N> ProviderLayer<P, N> for Identity
where
    N: Network,
    P: Provider<N>,
{
    type Provider = P;

    fn layer(&self, inner: P) -> Self::Provider {
        inner
    }
}

/// A stack of two providers.
#[derive(Debug)]
pub struct Stack<Inner, Outer> {
    inner: Inner,
    outer: Outer,
}

impl<Inner, Outer> Stack<Inner, Outer> {
    /// Create a new `Stack`.
    pub const fn new(inner: Inner, outer: Outer) -> Self {
        Self { inner, outer }
    }
}

impl<P, N, Inner, Outer> ProviderLayer<P, N> for Stack<Inner, Outer>
where
    N: Network,
    P: Provider<N>,
    Inner: ProviderLayer<P, N>,
    Outer: ProviderLayer<Inner::Provider, N>,
{
    type Provider = Outer::Provider;

    fn layer(&self, provider: P) -> Self::Provider {
        let inner = self.inner.layer(provider);

        self.outer.layer(inner)
    }
}

/// A builder for constructing a [`Provider`] from various layers.
///
/// This type is similar to [`tower::ServiceBuilder`], with extra complication
/// around maintaining the network and transport types.
///
/// The [`ProviderBuilder`] can be instantiated in two ways, using `ProviderBuilder::new()` or
/// `ProviderBuilder::default()`.
///
/// `ProviderBuilder::new()` will create a new [`ProviderBuilder`] with the [`RecommendedFillers`]
/// enabled, whereas `ProviderBuilder::default()` will instantiate it in its vanilla
/// [`ProviderBuilder`] form i.e with no fillers enabled.
///
/// [`tower::ServiceBuilder`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
#[derive(Debug)]
pub struct ProviderBuilder<L, F, N = Ethereum> {
    layer: L,
    filler: F,
    network: PhantomData<fn() -> N>,
}

impl
    ProviderBuilder<
        Identity,
        JoinFill<Identity, <Ethereum as RecommendedFillers>::RecommendedFillers>,
        Ethereum,
    >
{
    /// Create a new [`ProviderBuilder`] with the recommended filler enabled.
    ///
    /// Recommended fillers are preconfigured set of fillers that handle gas estimation, nonce
    /// management, and chain-id fetching.
    ///
    /// Building a provider with this setting enabled will return a [`crate::fillers::FillProvider`]
    /// with [`crate::utils::JoinedRecommendedFillers`].
    ///
    /// You can opt-out of using these fillers by using the `.disable_recommended_fillers()` method.
    pub fn new() -> Self {
        ProviderBuilder::default().with_recommended_fillers()
    }

    /// Opt-out of the recommended fillers by resetting the fillers stack in the
    /// [`ProviderBuilder`].
    ///
    /// This is equivalent to creating the builder using `ProviderBuilder::default()`.
    pub fn disable_recommended_fillers(self) -> ProviderBuilder<Identity, Identity, Ethereum> {
        ProviderBuilder { layer: self.layer, filler: Identity, network: self.network }
    }
}

impl<N> Default for ProviderBuilder<Identity, Identity, N> {
    fn default() -> Self {
        Self { layer: Identity, filler: Identity, network: PhantomData }
    }
}

impl ProviderBuilder<Identity, Identity, Ethereum> {
    /// Create a new [`ProviderBuilder`] with the [`RecommendedFillers`] for the provided
    /// [`Network`].
    pub fn new_with_network<Net: RecommendedFillers>(
    ) -> ProviderBuilder<Identity, JoinFill<Identity, Net::RecommendedFillers>, Net> {
        ProviderBuilder {
            layer: Identity,
            filler: JoinFill::new(Identity, Net::recommended_fillers()),
            network: PhantomData,
        }
    }
}

impl<L, N: Network> ProviderBuilder<L, Identity, N> {
    /// Add preconfigured set of layers handling gas estimation, nonce
    /// management, and chain-id fetching.
    pub fn with_recommended_fillers(
        self,
    ) -> ProviderBuilder<L, JoinFill<Identity, N::RecommendedFillers>, N>
    where
        N: RecommendedFillers,
    {
        self.filler(N::recommended_fillers())
    }
}

impl<L, F, N> ProviderBuilder<L, F, N> {
    /// Add a layer to the stack being built. This is similar to
    /// [`tower::ServiceBuilder::layer`].
    ///
    /// ## Note:
    ///
    /// Layers are added in outer-to-inner order, as in
    /// [`tower::ServiceBuilder`]. The first layer added will be the first to
    /// see the request.
    ///
    /// [`tower::ServiceBuilder::layer`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html#method.layer
    /// [`tower::ServiceBuilder`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
    pub fn layer<Inner>(self, layer: Inner) -> ProviderBuilder<Stack<Inner, L>, F, N> {
        ProviderBuilder {
            layer: Stack::new(layer, self.layer),
            filler: self.filler,
            network: PhantomData,
        }
    }

    /// Add a transaction filler to the stack being built. Transaction fillers
    /// are used to fill in missing fields on transactions before they are sent,
    /// and are all joined to form the outermost layer of the stack.
    pub fn filler<F2>(self, filler: F2) -> ProviderBuilder<L, JoinFill<F, F2>, N> {
        ProviderBuilder {
            layer: self.layer,
            filler: JoinFill::new(self.filler, filler),
            network: PhantomData,
        }
    }

    /// Change the network.
    ///
    /// By default, the network is `Ethereum`. This method must be called to configure a different
    /// network.
    ///
    /// ```ignore
    /// builder.network::<Arbitrum>()
    /// ```
    pub fn network<Net: Network>(self) -> ProviderBuilder<L, F, Net> {
        ProviderBuilder { layer: self.layer, filler: self.filler, network: PhantomData }
    }

    /// Add a chain layer to the stack being built. The layer will set
    /// the client's poll interval based on the average block time for this chain.
    ///
    /// Does nothing to the client with a local transport.
    pub fn with_chain(self, chain: NamedChain) -> ProviderBuilder<Stack<ChainLayer, L>, F, N> {
        self.layer(ChainLayer::new(chain))
    }

    // --- Fillers ---

    /// Add blob gas estimation to the stack being built.
    ///
    /// See [`BlobGasFiller`] for more information.
    pub fn with_blob_gas_estimation(self) -> ProviderBuilder<L, JoinFill<F, BlobGasFiller>, N> {
        self.filler(BlobGasFiller::default())
    }

    /// Add blob gas estimation to the stack being built, using the provided estimator.
    ///
    /// See [`BlobGasFiller`] and [`BlobGasEstimator`] for more information.
    pub fn with_blob_gas_estimator(
        self,
        estimator: BlobGasEstimator,
    ) -> ProviderBuilder<L, JoinFill<F, BlobGasFiller>, N> {
        self.filler(BlobGasFiller { estimator })
    }

    /// Add gas estimation to the stack being built.
    ///
    /// See [`GasFiller`] for more information.
    pub fn with_gas_estimation(self) -> ProviderBuilder<L, JoinFill<F, GasFiller>, N> {
        self.filler(GasFiller)
    }

    /// Add nonce management to the stack being built.
    ///
    /// See [`NonceFiller`] for more information.
    pub fn with_nonce_management<M: NonceManager>(
        self,
        nonce_manager: M,
    ) -> ProviderBuilder<L, JoinFill<F, NonceFiller<M>>, N> {
        self.filler(NonceFiller::new(nonce_manager))
    }

    /// Add simple nonce management to the stack being built.
    ///
    /// See [`SimpleNonceManager`] for more information.
    pub fn with_simple_nonce_management(
        self,
    ) -> ProviderBuilder<L, JoinFill<F, NonceFiller<SimpleNonceManager>>, N> {
        self.with_nonce_management(SimpleNonceManager::default())
    }

    /// Add cached nonce management to the stack being built.
    ///
    /// See [`CachedNonceManager`] for more information.
    pub fn with_cached_nonce_management(
        self,
    ) -> ProviderBuilder<L, JoinFill<F, NonceFiller<CachedNonceManager>>, N> {
        self.with_nonce_management(CachedNonceManager::default())
    }

    /// Add a chain ID filler to the stack being built. The filler will attempt
    /// to fetch the chain ID from the provider using
    /// [`Provider::get_chain_id`]. the first time a transaction is prepared,
    /// and will cache it for future transactions.
    pub fn fetch_chain_id(self) -> ProviderBuilder<L, JoinFill<F, ChainIdFiller>, N> {
        self.filler(ChainIdFiller::default())
    }

    /// Add a specific chain ID to the stack being built. The filler will
    /// fill transactions with the provided chain ID, regardless of the chain ID
    /// that the provider reports via [`Provider::get_chain_id`].
    pub fn with_chain_id(
        self,
        chain_id: ChainId,
    ) -> ProviderBuilder<L, JoinFill<F, ChainIdFiller>, N> {
        self.filler(ChainIdFiller::new(Some(chain_id)))
    }

    /// Add a wallet layer to the stack being built.
    ///
    /// See [`WalletFiller`].
    pub fn wallet<W: IntoWallet<N>>(
        self,
        wallet: W,
    ) -> ProviderBuilder<L, JoinFill<F, WalletFiller<W::NetworkWallet>>, N>
    where
        N: Network,
    {
        self.filler(WalletFiller::new(wallet.into_wallet()))
    }

    // --- Layers ---

    /// Aggregate multiple `eth_call` requests into a single batch request using Multicall3.
    ///
    /// See [`CallBatchLayer`] for more information.
    pub fn with_call_batching(self) -> ProviderBuilder<Stack<CallBatchLayer, L>, F, N> {
        self.layer(CallBatchLayer::new())
    }

    /// Aggregate multiple `eth_call` requests with block number queries done by calling Arbsym
    /// precompile.
    ///
    /// See [`CallBatchLayer`] for more information.
    pub fn with_arbitrum_call_batching(self) -> ProviderBuilder<Stack<CallBatchLayer, L>, F, N> {
        self.layer(CallBatchLayer::new().arbitrum_compat())
    }

    /// Add response caching to the stack being built with the specified maximum cache size.
    ///
    /// See [`CacheLayer`](crate::layers::CacheLayer) for more information.
    #[cfg(not(target_family = "wasm"))]
    pub fn with_caching(
        self,
        max_items: u32,
    ) -> ProviderBuilder<Stack<crate::layers::CacheLayer, L>, F, N> {
        self.layer(crate::layers::CacheLayer::new(max_items))
    }

    /// Add response caching to the stack being built with a default cache size of 100 items.
    ///
    /// See [`CacheLayer`](crate::layers::CacheLayer) for more information.
    #[cfg(not(target_family = "wasm"))]
    pub fn with_default_caching(
        self,
    ) -> ProviderBuilder<Stack<crate::layers::CacheLayer, L>, F, N> {
        self.with_caching(100)
    }

    // --- Build to Provider ---

    /// Finish the layer stack by providing a root [`Provider`], outputting
    /// the final [`Provider`] type with all stack components.
    pub fn connect_provider<P>(self, provider: P) -> F::Provider
    where
        L: ProviderLayer<P, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        P: Provider<N>,
        N: Network,
    {
        let Self { layer, filler, network: PhantomData } = self;
        let stack = Stack::new(layer, filler);
        stack.layer(provider)
    }

    /// Finish the layer stack by providing a root [`RpcClient`], outputting
    /// the final [`Provider`] type with all stack components.
    ///
    /// This is a convenience function for
    /// `ProviderBuilder::on_provider(RootProvider::new(client))`.
    pub fn connect_client(self, client: RpcClient) -> F::Provider
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        self.connect_provider(RootProvider::new(client))
    }

    /// Finish the layer stack by providing a [`RpcClient`] that mocks responses, outputting
    /// the final [`Provider`] type with all stack components.
    ///
    /// This is a convenience function for
    /// `ProviderBuilder::on_client(RpcClient::mocked(asserter))`.
    pub fn connect_mocked_client(self, asserter: alloy_transport::mock::Asserter) -> F::Provider
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        self.connect_client(RpcClient::mocked(asserter))
    }

    /// Finish the layer stack by providing a connection string for a built-in
    /// transport type, outputting the final [`Provider`] type with all stack
    /// components.
    #[doc(alias = "on_builtin")]
    pub async fn connect(self, s: &str) -> Result<F::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().connect(s).await?;
        Ok(self.connect_client(client))
    }

    /// Finish the layer stack by providing a connection string with custom configuration.
    ///
    /// This method allows for fine-grained control over connection settings
    /// such as authentication, retry behavior, and transport-specific options.
    /// The transport type is extracted from the connection string and configured
    /// using the provided [`ConnectionConfig`].
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_provider::{ConnectionConfig, ProviderBuilder};
    /// use alloy_transport::Authorization;
    /// use std::time::Duration;
    ///
    /// let config = ConnectionConfig::new()
    ///     .with_auth(Authorization::bearer("my-token"))
    ///     .with_max_retries(3)
    ///     .with_retry_interval(Duration::from_secs(2));
    ///
    /// let provider =
    ///     ProviderBuilder::new().connect_with_config("ws://localhost:8545", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_with_config(
        self,
        s: &str,
        config: ConnectionConfig,
    ) -> Result<F::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().connect_with_config(s, config).await?;
        Ok(self.connect_client(client))
    }

    /// Finish the layer stack by providing a [`TransportConnect`] instance.
    pub async fn connect_with<C>(self, connect: &C) -> Result<F::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
        C: TransportConnect,
    {
        connect
            .get_transport()
            .await
            .map(|t| RpcClient::new(t, connect.is_local()))
            .map(|client| self.connect_client(client))
    }

    /// Finish the layer stack by providing a [`PubSubConnect`] instance,
    /// producing a [`Provider`] with pubsub capabilities.
    ///
    /// [`PubSubConnect`]: alloy_pubsub::PubSubConnect
    #[cfg(feature = "pubsub")]
    pub async fn connect_pubsub_with<C>(self, connect: C) -> Result<F::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
        C: alloy_pubsub::PubSubConnect,
    {
        ClientBuilder::default().pubsub(connect).await.map(|client| self.connect_client(client))
    }

    /// Build this provider with a websocket connection.
    #[cfg(feature = "ws")]
    pub async fn connect_ws(
        self,
        connect: alloy_transport_ws::WsConnect,
    ) -> Result<F::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().ws(connect).await?;
        Ok(self.connect_client(client))
    }

    /// Build this provider with an IPC connection.
    #[cfg(feature = "ipc")]
    pub async fn connect_ipc<T>(
        self,
        connect: alloy_transport_ipc::IpcConnect<T>,
    ) -> Result<F::Provider, TransportError>
    where
        alloy_transport_ipc::IpcConnect<T>: alloy_pubsub::PubSubConnect,
        L: ProviderLayer<RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().ipc(connect).await?;
        Ok(self.connect_client(client))
    }

    /// Build this provider with an Reqwest HTTP transport.
    #[cfg(any(test, feature = "reqwest"))]
    pub fn connect_http(self, url: reqwest::Url) -> F::Provider
    where
        L: ProviderLayer<crate::RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().http(url);
        self.connect_client(client)
    }

    /// Build this provider with a pre-built Reqwest client.
    #[cfg(any(test, feature = "reqwest"))]
    pub fn connect_reqwest<C>(self, client: C, url: reqwest::Url) -> F::Provider
    where
        L: ProviderLayer<crate::RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
        C: Into<reqwest::Client>,
    {
        let client = ClientBuilder::default().http_with_client(client.into(), url);
        self.connect_client(client)
    }

    /// Build this provider with a provided Reqwest client builder.
    #[cfg(any(test, feature = "reqwest"))]
    pub fn with_reqwest<B>(self, url: reqwest::Url, builder: B) -> F::Provider
    where
        L: ProviderLayer<crate::RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
        B: FnOnce(reqwest::ClientBuilder) -> reqwest::Client,
    {
        self.connect_reqwest(builder(reqwest::ClientBuilder::default()), url)
    }

    /// Build this provider with an Hyper HTTP transport.
    #[cfg(feature = "hyper")]
    pub fn connect_hyper_http(self, url: url::Url) -> F::Provider
    where
        L: ProviderLayer<crate::RootProvider<N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        N: Network,
    {
        let client = ClientBuilder::default().hyper_http(url);
        self.connect_client(client)
    }
}

#[cfg(any(test, feature = "anvil-node"))]
type JoinedEthereumWalletFiller<F> = JoinFill<F, WalletFiller<alloy_network::EthereumWallet>>;

#[cfg(any(test, feature = "anvil-node"))]
type AnvilProviderResult<T> = Result<T, alloy_node_bindings::NodeError>;

#[cfg(any(test, feature = "anvil-node"))]
impl<L, F, N: Network> ProviderBuilder<L, F, N> {
    /// Build this provider with anvil, using the BoxTransport.
    pub fn connect_anvil(self) -> F::Provider
    where
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        L: crate::builder::ProviderLayer<
            crate::layers::AnvilProvider<crate::provider::RootProvider<N>, N>,
            N,
        >,
    {
        self.connect_anvil_with_config(std::convert::identity)
    }

    /// Build this provider with anvil, using the BoxTransport. This
    /// function configures a wallet backed by anvil keys, and is intended for
    /// use in tests.
    pub fn connect_anvil_with_wallet(
        self,
    ) -> <JoinedEthereumWalletFiller<F> as ProviderLayer<L::Provider, N>>::Provider
    where
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        L: crate::builder::ProviderLayer<
            crate::layers::AnvilProvider<crate::provider::RootProvider<N>, N>,
            N,
        >,
        alloy_network::EthereumWallet: alloy_network::NetworkWallet<N>,
    {
        self.connect_anvil_with_wallet_and_config(std::convert::identity)
            .expect("failed to build provider")
    }

    /// Build this provider with anvil, using the BoxTransport. The
    /// given function is used to configure the anvil instance.
    pub fn connect_anvil_with_config(
        self,
        f: impl FnOnce(alloy_node_bindings::Anvil) -> alloy_node_bindings::Anvil,
    ) -> F::Provider
    where
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        L: crate::builder::ProviderLayer<
            crate::layers::AnvilProvider<crate::provider::RootProvider<N>, N>,
            N,
        >,
    {
        let anvil_layer = crate::layers::AnvilLayer::from(f(Default::default()));
        let url = anvil_layer.endpoint_url();

        let rpc_client = ClientBuilder::default().http(url);

        self.layer(anvil_layer).connect_client(rpc_client)
    }

    /// Build this provider with anvil, using the BoxTransport. The
    /// given function is used to configure the anvil instance.
    #[deprecated(since = "0.12.6", note = "use `connect_anvil_with_config` instead")]
    pub fn on_anvil_with_config(
        self,
        f: impl FnOnce(alloy_node_bindings::Anvil) -> alloy_node_bindings::Anvil,
    ) -> F::Provider
    where
        L: ProviderLayer<crate::layers::AnvilProvider<RootProvider<N>, N>, N>,
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
    {
        let anvil_layer = crate::layers::AnvilLayer::from(f(Default::default()));
        let url = anvil_layer.endpoint_url();

        let rpc_client = ClientBuilder::default().http(url);

        self.layer(anvil_layer).connect_client(rpc_client)
    }

    /// Build this provider with anvil, using the BoxTransport.
    /// This calls `try_on_anvil_with_wallet_and_config` and panics on error.
    pub fn connect_anvil_with_wallet_and_config(
        self,
        f: impl FnOnce(alloy_node_bindings::Anvil) -> alloy_node_bindings::Anvil,
    ) -> AnvilProviderResult<
        <JoinedEthereumWalletFiller<F> as ProviderLayer<L::Provider, N>>::Provider,
    >
    where
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        L: crate::builder::ProviderLayer<
            crate::layers::AnvilProvider<crate::provider::RootProvider<N>, N>,
            N,
        >,
        alloy_network::EthereumWallet: alloy_network::NetworkWallet<N>,
    {
        let anvil_layer = crate::layers::AnvilLayer::from(f(Default::default()));
        let url = anvil_layer.endpoint_url();

        let wallet = anvil_layer
            .instance()
            .wallet()
            .ok_or(alloy_node_bindings::NodeError::NoKeysAvailable)?;

        let rpc_client = ClientBuilder::default().http(url);

        Ok(self.wallet(wallet).layer(anvil_layer).connect_client(rpc_client))
    }

    /// Build this provider with anvil, using the BoxTransport.
    /// This calls `try_on_anvil_with_wallet_and_config` and panics on error.
    #[deprecated(since = "0.12.6", note = "use `connect_anvil_with_wallet_and_config` instead")]
    pub fn on_anvil_with_wallet_and_config(
        self,
        f: impl FnOnce(alloy_node_bindings::Anvil) -> alloy_node_bindings::Anvil,
    ) -> AnvilProviderResult<
        <JoinedEthereumWalletFiller<F> as ProviderLayer<L::Provider, N>>::Provider,
    >
    where
        F: TxFiller<N> + ProviderLayer<L::Provider, N>,
        L: crate::builder::ProviderLayer<
            crate::layers::AnvilProvider<crate::provider::RootProvider<N>, N>,
            N,
        >,
        alloy_network::EthereumWallet: alloy_network::NetworkWallet<N>,
    {
        let anvil_layer = crate::layers::AnvilLayer::from(f(Default::default()));
        let url = anvil_layer.endpoint_url();

        let wallet = anvil_layer
            .instance()
            .wallet()
            .ok_or(alloy_node_bindings::NodeError::NoKeysAvailable)?;

        let rpc_client = ClientBuilder::default().http(url);

        Ok(self.wallet(wallet).layer(anvil_layer).connect_client(rpc_client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;
    use alloy_network::AnyNetwork;

    #[tokio::test]
    async fn basic() {
        let provider = ProviderBuilder::new()
            .with_cached_nonce_management()
            .with_call_batching()
            .connect_http("http://localhost:8545".parse().unwrap());
        let _ = provider.get_account(Default::default());
        let provider = provider.erased();
        let _ = provider.get_account(Default::default());
    }

    #[tokio::test]
    #[cfg(feature = "reqwest")]
    async fn test_connect_reqwest() {
        let provider = ProviderBuilder::new()
            .with_cached_nonce_management()
            .with_call_batching()
            .connect_reqwest(
                reqwest::Client::new(),
                reqwest::Url::parse("http://localhost:8545").unwrap(),
            );
        let _ = provider.get_account(Default::default());
        let provider = provider.erased();
        let _ = provider.get_account(Default::default());
    }

    #[tokio::test]
    #[cfg(feature = "reqwest")]
    async fn test_with_reqwest() {
        let provider = ProviderBuilder::new()
            .with_cached_nonce_management()
            .with_call_batching()
            .with_reqwest(reqwest::Url::parse("http://localhost:8545").unwrap(), |builder| {
                builder
                    .user_agent("alloy/test")
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .expect("failed to build reqwest client")
            });
        let _ = provider.get_account(Default::default());
        let provider = provider.erased();
        let _ = provider.get_account(Default::default());
    }

    #[tokio::test]
    async fn compile_with_network() {
        let p = ProviderBuilder::new_with_network::<AnyNetwork>().connect_anvil();
        let num = p.get_block_number().await.unwrap();
        assert_eq!(num, 0);
    }
}
