use crate::{
    blocks::NewBlocks,
    heart::{Heartbeat, HeartbeatHandle},
    Identity, ProviderBuilder,
};
use alloy_network::{Ethereum, Network};
use alloy_rpc_client::{BuiltInConnectionString, ClientBuilder, ClientRef, RpcClient, WeakClient};
use alloy_transport::{BoxTransport, BoxTransportConnect, Transport, TransportError};
use std::{
    fmt,
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

#[cfg(feature = "reqwest")]
use alloy_transport_http::Http;

#[cfg(feature = "pubsub")]
use alloy_pubsub::{PubSubFrontend, Subscription};

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<T, N: Network = Ethereum> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<T, N>>,
}

impl<T, N: Network> Clone for RootProvider<T, N> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: fmt::Debug, N: Network> fmt::Debug for RootProvider<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootProvider").field("client", &self.inner.client).finish_non_exhaustive()
    }
}

/// Helper function to directly access [`ProviderBuilder`] with minimal
/// generics.
pub fn builder<N: Network>() -> ProviderBuilder<Identity, Identity, N> {
    ProviderBuilder::default()
}

#[cfg(feature = "reqwest")]
impl<N: Network> RootProvider<Http<reqwest::Client>, N> {
    /// Creates a new HTTP root provider from the given URL.
    pub fn new_http(url: url::Url) -> Self {
        Self::new(RpcClient::new_http(url))
    }
}

impl<T: Transport + Clone, N: Network> RootProvider<T, N> {
    /// Creates a new root provider from the given RPC client.
    pub fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network> RootProvider<BoxTransport, N> {
    /// Connects to a boxed transport with the given connector.
    pub async fn connect_boxed<C: BoxTransportConnect>(conn: C) -> Result<Self, TransportError> {
        let client = ClientBuilder::default().connect_boxed(conn).await?;
        Ok(Self::new(client))
    }

    /// Creates a new root provider from the provided connection details.
    pub async fn connect_builtin(s: &str) -> Result<Self, TransportError> {
        let conn: BuiltInConnectionString = s.parse()?;

        let client = ClientBuilder::default().connect_boxed(conn).await?;
        Ok(Self::new(client))
    }
}

impl<T: Transport + Clone, N: Network> RootProvider<T, N> {
    /// Boxes the inner client.
    ///
    /// This will create a new provider if this instance is not the only reference to the inner
    /// client.
    pub fn boxed(self) -> RootProvider<BoxTransport, N> {
        let inner = Arc::unwrap_or_clone(self.inner);
        RootProvider { inner: Arc::new(inner.boxed()) }
    }

    /// Gets the subscription corresponding to the given RPC subscription ID.
    #[cfg(feature = "pubsub")]
    pub async fn get_subscription<R: alloy_json_rpc::RpcReturn>(
        &self,
        id: alloy_primitives::B256,
    ) -> alloy_transport::TransportResult<Subscription<R>> {
        self.pubsub_frontend()?.get_subscription(id).await.map(Subscription::from)
    }

    /// Unsubscribes from the subscription corresponding to the given RPC subscription ID.
    #[cfg(feature = "pubsub")]
    pub fn unsubscribe(&self, id: alloy_primitives::B256) -> alloy_transport::TransportResult<()> {
        self.pubsub_frontend()?.unsubscribe(id)
    }

    #[cfg(feature = "pubsub")]
    pub(crate) fn pubsub_frontend(&self) -> alloy_transport::TransportResult<&PubSubFrontend> {
        self.inner
            .client_ref()
            .pubsub_frontend()
            .ok_or_else(alloy_transport::TransportErrorKind::pubsub_unavailable)
    }

    #[inline]
    pub(crate) fn get_heart(&self) -> &HeartbeatHandle<N> {
        self.inner.heart.get_or_init(|| {
            let new_blocks = NewBlocks::<T, N>::new(self.inner.weak_client());
            let stream = new_blocks.into_stream();
            Heartbeat::new(Box::pin(stream)).spawn()
        })
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub(crate) struct RootProviderInner<T, N: Network = Ethereum> {
    client: RpcClient<T>,
    heart: OnceLock<HeartbeatHandle<N>>,
    _network: PhantomData<N>,
}

impl<T, N: Network> Clone for RootProviderInner<T, N> {
    fn clone(&self) -> Self {
        Self { client: self.client.clone(), heart: self.heart.clone(), _network: PhantomData }
    }
}

impl<T: Transport + Clone, N: Network> RootProviderInner<T, N> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: Default::default(), _network: PhantomData }
    }

    pub(crate) fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    pub(crate) fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }
}

impl<T: Transport + Clone, N: Network> RootProviderInner<T, N> {
    fn boxed(self) -> RootProviderInner<BoxTransport, N> {
        RootProviderInner { client: self.client.boxed(), heart: self.heart, _network: PhantomData }
    }
}
