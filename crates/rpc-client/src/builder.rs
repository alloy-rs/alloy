use crate::RpcClient;
use alloy_transport::{
    BoxTransport, BoxTransportConnect, Transport, TransportConnect, TransportResult,
};
use tower::{
    layer::util::{Identity, Stack},
    Layer, ServiceBuilder,
};

/// A builder for the transport  [`RpcClient`].
///
/// This is a wrapper around [`tower::ServiceBuilder`]. It allows you to
/// configure middleware layers that will be applied to the transport, and has
/// some shortcuts for common layers and transports.
///
/// A builder accumulates Layers, and then is finished via the
/// [`ClientBuilder::connect`] method, which produces an RPC client.
#[derive(Debug)]
pub struct ClientBuilder<L> {
    pub(crate) builder: ServiceBuilder<L>,
}

impl Default for ClientBuilder<Identity> {
    fn default() -> Self {
        Self { builder: ServiceBuilder::new() }
    }
}

impl<L> ClientBuilder<L> {
    /// Add a middleware layer to the stack.
    ///
    /// This is a wrapper around [`tower::ServiceBuilder::layer`]. Layers that
    /// are added first will be called with the request first.
    pub fn layer<M>(self, layer: M) -> ClientBuilder<Stack<M, L>> {
        ClientBuilder { builder: self.builder.layer(layer) }
    }

    /// Create a new [`RpcClient`] with the given transport and the configured
    /// layers.
    pub fn transport<T>(self, transport: T, is_local: bool) -> RpcClient<L::Service>
    where
        L: Layer<T>,
        T: Transport,
        L::Service: Transport,
    {
        RpcClient::new(self.builder.service(transport), is_local)
    }

    /// Convenience function to create a new [`RpcClient`] with a [`reqwest`]
    /// HTTP transport.
    #[cfg(feature = "reqwest")]
    pub fn http(self, url: url::Url) -> RpcClient<L::Service>
    where
        L: Layer<alloy_transport_http::Http<reqwest::Client>>,
        L::Service: Transport,
    {
        let transport = alloy_transport_http::Http::new(url);
        let is_local = transport.guess_local();

        self.transport(transport, is_local)
    }

    /// Convenience function to create a new [`RpcClient`] with a `hyper` HTTP transport.
    #[cfg(all(not(target_arch = "wasm32"), feature = "hyper"))]
    pub fn hyper_http(self, url: url::Url) -> RpcClient<L::Service>
    where
        L: Layer<alloy_transport_http::Http<alloy_transport_http::HyperClient>>,
        L::Service: Transport,
    {
        let executor = hyper_util::rt::TokioExecutor::new();
        let client = hyper_util::client::legacy::Client::builder(executor).build_http();
        let transport = alloy_transport_http::Http::with_client(client, url);
        let is_local = transport.guess_local();

        self.transport(transport, is_local)
    }

    /// Connect a pubsub transport, producing an [`RpcClient`] with the provided
    /// connection.
    #[cfg(feature = "pubsub")]
    pub async fn pubsub<C>(self, pubsub_connect: C) -> TransportResult<RpcClient<L::Service>>
    where
        C: alloy_pubsub::PubSubConnect,
        L: Layer<alloy_pubsub::PubSubFrontend>,
        L::Service: Transport,
    {
        let is_local = pubsub_connect.is_local();
        let transport = pubsub_connect.into_service().await?;
        Ok(self.transport(transport, is_local))
    }

    /// Connect a WS transport, producing an [`RpcClient`] with the provided
    /// connection
    #[cfg(feature = "ws")]
    pub async fn ws(
        self,
        ws_connect: alloy_transport_ws::WsConnect,
    ) -> TransportResult<RpcClient<L::Service>>
    where
        L: Layer<alloy_pubsub::PubSubFrontend>,
        L::Service: Transport,
    {
        self.pubsub(ws_connect).await
    }

    /// Connect an IPC transport, producing an [`RpcClient`] with the provided
    /// connection.
    #[cfg(feature = "ipc")]
    pub async fn ipc<T>(
        self,
        ipc_connect: alloy_transport_ipc::IpcConnect<T>,
    ) -> TransportResult<RpcClient<L::Service>>
    where
        alloy_transport_ipc::IpcConnect<T>: alloy_pubsub::PubSubConnect,
        L: Layer<alloy_pubsub::PubSubFrontend>,
        L::Service: Transport,
    {
        self.pubsub(ipc_connect).await
    }

    /// Connect a transport, producing an [`RpcClient`] with the provided
    /// connection.
    pub async fn connect<C>(self, connect: C) -> TransportResult<RpcClient<L::Service>>
    where
        C: TransportConnect,
        L: Layer<C::Transport>,
        L::Service: Transport,
    {
        let transport = connect.get_transport().await?;
        Ok(self.transport(transport, connect.is_local()))
    }

    /// Connect a transport, producing an [`RpcClient`] with a [`BoxTransport`]
    /// connection.
    pub async fn connect_boxed<C>(self, connect: C) -> TransportResult<RpcClient<L::Service>>
    where
        C: BoxTransportConnect,
        L: Layer<BoxTransport>,
        L::Service: Transport,
    {
        let transport = connect.get_boxed_transport().await?;
        Ok(self.transport(transport, connect.is_local()))
    }
}
