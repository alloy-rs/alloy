use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_transport::Transport;
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};
use url::Url;

use crate::{Provider, ProviderLayer, RootProvider};

/// A layer that wraps an [`Anvil`] config. The config will be used
/// to spawn an [`AnvilInstance`] when the layer is applied, or when the user
/// requests any information about the anvil node (e.g. via the
/// [`AnvilLayer::ws_endpoint_url`] method ).
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

impl<P, T> ProviderLayer<P, T, Ethereum> for AnvilLayer
where
    P: Provider<T>,
    T: Transport + Clone,
{
    type Provider = AnvilProvider<P, T>;

    fn layer(&self, inner: P) -> Self::Provider {
        let anvil = self.instance();
        AnvilProvider::new(inner, anvil.clone())
    }
}

/// A provider that wraps an [`AnvilInstance`], preventing the instance from
/// being dropped while the provider is in use.
#[derive(Clone, Debug)]
pub struct AnvilProvider<P, T> {
    inner: P,
    _anvil: Arc<AnvilInstance>,
    _pd: PhantomData<fn() -> T>,
}

impl<P, T> AnvilProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    /// Creates a new `AnvilProvider` with the given inner provider and anvil
    /// instance.
    pub fn new(inner: P, _anvil: Arc<AnvilInstance>) -> Self {
        Self { inner, _anvil, _pd: PhantomData }
    }
}

impl<P, T> Provider<T> for AnvilProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    #[inline(always)]
    fn root(&self) -> &RootProvider<T> {
        self.inner.root()
    }
}
