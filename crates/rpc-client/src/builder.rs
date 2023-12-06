use crate::RpcClient;
use alloy_transport::{
    BoxTransport, BoxTransportConnect, Transport, TransportConnect, TransportError,
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
    fn transport<T>(self, transport: T, is_local: bool) -> RpcClient<L::Service>
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
    pub fn reqwest_http(self, url: url::Url) -> RpcClient<L::Service>
    where
        L: Layer<alloy_transport_http::Http<reqwest::Client>>,
        L::Service: Transport,
    {
        let transport = alloy_transport_http::Http::new(url);
        let is_local = transport.guess_local();

        self.transport(transport, is_local)
    }

    /// Convenience function to create a new [`RpcClient`] with a [`hyper`]
    /// HTTP transport.
    #[cfg(all(not(target_arch = "wasm32"), feature = "hyper"))]
    pub fn hyper_http(self, url: url::Url) -> RpcClient<L::Service>
    where
        L: Layer<alloy_transport_http::Http<hyper::client::Client<hyper::client::HttpConnector>>>,
        L::Service: Transport,
    {
        let transport = alloy_transport_http::Http::new(url);
        let is_local = transport.guess_local();

        self.transport(transport, is_local)
    }

    #[cfg(feature = "pubsub")]
    /// Connect a pubsub transport, producing an [`RpcClient`] with the provided
    /// connection.
    pub async fn pubsub<C>(self, pubsub_connect: C) -> Result<RpcClient<L::Service>, TransportError>
    where
        C: alloy_pubsub::PubSubConnect,
        L: Layer<alloy_pubsub::PubSubFrontend>,
        L::Service: Transport,
    {
        let is_local = pubsub_connect.is_local();
        let transport = pubsub_connect.into_service().await?;
        Ok(self.transport(transport, is_local))
    }

    #[cfg(feature = "ws")]
    /// Connect a WS transport, producing an [`RpcClient`] with the provided
    /// connection
    pub async fn ws(
        self,
        ws_connect: alloy_transport_ws::WsConnect,
    ) -> Result<RpcClient<L::Service>, TransportError>
    where
        L: Layer<alloy_pubsub::PubSubFrontend>,
        L::Service: Transport,
    {
        self.pubsub(ws_connect).await
    }

    /// Connect a transport, producing an [`RpcClient`] with the provided
    /// connection.
    pub async fn connect<C>(self, connect: C) -> Result<RpcClient<L::Service>, TransportError>
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
    pub async fn connect_boxed<C>(self, connect: C) -> Result<RpcClient<L::Service>, TransportError>
    where
        C: BoxTransportConnect,
        L: Layer<BoxTransport>,
        L::Service: Transport,
    {
        let transport = connect.get_boxed_transport().await?;
        Ok(self.transport(transport, connect.is_local()))
    }
}
