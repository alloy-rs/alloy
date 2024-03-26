use crate::{
    layers::{GasEstimatorLayer, ManagedNonceLayer, SignerLayer},
    Provider, RootProvider,
};
use alloy_network::{Ethereum, Network};
use alloy_rpc_client::{BuiltInConnectionString, ClientBuilder, RpcClient};
use alloy_transport::{BoxTransport, Transport, TransportError};
use std::marker::PhantomData;

/// A layering abstraction in the vein of [`tower::Layer`]
///
/// [`tower::Layer`]: https://docs.rs/tower/latest/tower/trait.Layer.html
pub trait ProviderLayer<P: Provider<N, T>, N: Network, T: Transport + Clone> {
    /// The provider constructed by this layer.
    type Provider: Provider<N, T>;

    /// Wrap the given provider in the layer's provider.
    fn layer(&self, inner: P) -> Self::Provider;
}

/// An identity layer that does nothing.
#[derive(Debug, Clone, Copy)]
pub struct Identity;

impl<P, N, T> ProviderLayer<P, N, T> for Identity
where
    T: Transport + Clone,
    N: Network,
    P: Provider<N, T>,
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
        Stack { inner, outer }
    }
}

impl<P, N, T, Inner, Outer> ProviderLayer<P, N, T> for Stack<Inner, Outer>
where
    T: Transport + Clone,
    N: Network,
    P: Provider<N, T>,
    Inner: ProviderLayer<P, N, T>,
    Outer: ProviderLayer<Inner::Provider, N, T>,
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
/// [`tower::ServiceBuilder`]: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
#[derive(Debug)]
pub struct ProviderBuilder<L, N = Ethereum> {
    layer: L,
    network: PhantomData<N>,
}

impl<N> ProviderBuilder<Identity, N> {
    /// Create a new [`ProviderBuilder`].
    pub const fn new() -> Self {
        ProviderBuilder { layer: Identity, network: PhantomData }
    }
}

impl<N> Default for ProviderBuilder<Identity, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L, N> ProviderBuilder<L, N> {
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
    pub fn layer<Inner>(self, layer: Inner) -> ProviderBuilder<Stack<Inner, L>, N> {
        ProviderBuilder { layer: Stack::new(layer, self.layer), network: PhantomData }
    }

    /// Add a signer layer to the stack being built.
    ///
    /// See [`SignerLayer`].
    pub fn signer<S>(self, signer: S) -> ProviderBuilder<Stack<SignerLayer<S>, L>, N> {
        self.layer(SignerLayer::new(signer))
    }

    /// Add gas estimation to the stack being built.
    ///
    /// See [`GasEstimatorLayer`]
    pub fn with_gas_estimation(self) -> ProviderBuilder<Stack<GasEstimatorLayer, L>, N> {
        self.layer(GasEstimatorLayer)
    }

    /// Add nonce management to the stack being built.
    ///
    /// See [`ManagedNonceLayer`]
    pub fn with_nonce_management(self) -> ProviderBuilder<Stack<ManagedNonceLayer, L>, N> {
        self.layer(ManagedNonceLayer)
    }

    /// Add preconfigured set of layers handling gas estimation and nonce management
    pub fn with_recommended_layers(
        self,
    ) -> ProviderBuilder<Stack<ManagedNonceLayer, Stack<GasEstimatorLayer, L>>, N> {
        self.with_gas_estimation().with_nonce_management()
    }

    /// Change the network.
    ///
    /// By default, the network is `Ethereum`. This method must be called to configure a different
    /// network.
    ///
    /// ```rust,ignore
    /// builder.network::<Arbitrum>()
    /// ```
    pub fn network<Net: Network>(self) -> ProviderBuilder<L, Net> {
        ProviderBuilder { layer: self.layer, network: PhantomData }
    }

    /// Finish the layer stack by providing a root [`Provider`], outputting
    /// the final [`Provider`] type with all stack components.
    pub fn provider<P, T>(self, provider: P) -> L::Provider
    where
        L: ProviderLayer<P, N, T>,
        P: Provider<N, T>,
        T: Transport + Clone,
        N: Network,
    {
        self.layer.layer(provider)
    }

    /// Finish the layer stack by providing a root [`RpcClient`], outputting
    /// the final [`Provider`] type with all stack components.
    ///
    /// This is a convenience function for
    /// `ProviderBuilder::provider<RpcClient>`.
    pub fn on_client<T>(self, client: RpcClient<T>) -> L::Provider
    where
        L: ProviderLayer<RootProvider<N, T>, N, T>,
        T: Transport + Clone,
        N: Network,
    {
        self.provider(RootProvider::new(client))
    }

    /// Finish the layer stack by providing a connection string for a built-in
    /// transport type, outputting the final [`Provider`] type with all stack
    /// components.
    ///
    /// This is a convenience function for
    pub async fn on_builtin(self, s: &str) -> Result<L::Provider, TransportError>
    where
        L: ProviderLayer<RootProvider<N, BoxTransport>, N, BoxTransport>,
        N: Network,
    {
        let connect: BuiltInConnectionString = s.parse()?;
        let client = ClientBuilder::default().connect_boxed(connect).await?;
        Ok(self.on_client(client))
    }
}

// Copyright (c) 2019 Tower Contributors

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
