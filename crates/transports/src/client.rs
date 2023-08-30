use alloy_json_rpc::{Id, Request, RpcParam, RpcReturn};
use serde_json::value::RawValue;
use tower::{layer::util::Stack, Layer, ServiceBuilder};

use std::{
    borrow::Cow,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{BatchRequest, BoxTransport, Http, RpcCall, Transport};

/// A JSON-RPC client.
///
/// This struct manages a [`Transport`] and a request ID counter. It is used to
/// build [`RpcCall`] and [`BatchRequest`] objects. The client delegates
/// transport access to the calls.
///
/// ### Note
///
/// IDs are allocated sequentially, starting at 0. IDs are reserved via
/// [`RpcClient::next_id`]. Note that allocated IDs may not be used. There is
/// no guarantee that a prepared [`RpcCall`] will be sent, or that a sent call
/// will receive a response.
#[derive(Debug)]
pub struct RpcClient<T> {
    /// The underlying transport.
    pub(crate) transport: T,
    /// `true` if the transport is local.
    pub(crate) is_local: bool,
    /// The next request ID to use.
    pub(crate) id: AtomicU64,
}

impl<T> RpcClient<T> {
    /// Create a new [`RpcClient`] with the given transport.
    pub fn new(t: T, is_local: bool) -> Self {
        Self {
            transport: t,
            is_local,
            id: AtomicU64::new(0),
        }
    }

    /// `true` if the client believes the transport is local.
    ///
    /// This can be used to optimize remote API usage, or to change program
    /// behavior on local endpoints. When the client is instantiated by parsing
    /// a URL or other external input, this value is set on a best-efforts
    /// basis and may be incorrect.
    #[inline]
    pub fn is_local(&self) -> bool {
        self.is_local
    }

    /// Set the `is_local` flag.
    pub fn set_local(&mut self, is_local: bool) {
        self.is_local = is_local;
    }

    /// Reserve a request ID value. This is used to generate request IDs.
    #[inline]
    fn increment_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    /// Reserve a request ID u64.
    #[inline]
    pub fn next_id(&self) -> Id {
        Id::Number(self.increment_id())
    }
}

impl<T> RpcClient<T>
where
    T: Transport + Clone,
    T::Future: Send,
{
    /// Create a new [`BatchRequest`] builder.
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<T> {
        BatchRequest::new(self)
    }

    /// Build a `JsonRpcRequest` with the given method and params.
    ///
    /// This function reserves an ID for the request, however the request
    /// is not sent. To send a request, use [`RpcClient::prepare`] and await
    /// the returned [`RpcCall`].
    pub fn make_request<'a, Params: RpcParam>(
        &self,
        method: &'static str,
        params: Cow<'a, Params>,
    ) -> Request<Cow<'a, Params>> {
        Request {
            method,
            params,
            id: self.next_id(),
        }
    }

    /// Prepare an [`RpcCall`].
    ///
    /// This function reserves an ID for the request, however the request
    /// is not sent. To send a request, await the returned [`RpcCall`].
    ///
    /// ### Note:
    ///
    /// Serialization is done lazily. It will not be performed until the call
    /// is awaited. This means that if a serializer error occurs, it will not
    /// be caught until the call is awaited.
    pub fn prepare<'a, Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Cow<'a, Params>,
    ) -> RpcCall<T, Cow<'a, Params>, Resp> {
        let request = self.make_request(method, params);
        RpcCall::new(request, self.transport.clone())
    }

    /// Type erase the service in the transport, allowing it to be used in a
    /// generic context.
    ///
    /// ## Note:
    ///
    /// This is for abstracting over `RpcClient<T>` for multiple `T` by
    /// erasing each type. E.g. if you have `RpcClient<Http>` and
    /// `RpcClient<Ws>` you can put both into a `Vec<RpcClient<BoxTransport>>`.
    #[inline]
    pub fn boxed(self) -> RpcClient<BoxTransport> {
        RpcClient {
            transport: self.transport.boxed(),
            is_local: self.is_local,
            id: self.id,
        }
    }
}

/// A builder for the transport  [`RpcClient`].
///
/// This is a wrapper around [`tower::ServiceBuilder`]. It allows you to
/// configure middleware layers that will be applied to the transport, and has
/// some shortcuts for common layers and transports.
pub struct ClientBuilder<L> {
    builder: ServiceBuilder<L>,
}

impl<L> ClientBuilder<L> {
    /// Add a middleware layer to the stack.
    ///
    /// This is a wrapper around [`tower::ServiceBuilder::layer`]. Layers that
    /// are added first will be called with the request first.
    pub fn layer<M>(self, layer: M) -> ClientBuilder<Stack<M, L>> {
        ClientBuilder {
            builder: self.builder.layer(layer),
        }
    }

    /// Create a new [`RpcClient`] with the given transport and the configured
    /// layers.
    pub fn transport<T>(self, transport: T, is_local: bool) -> RpcClient<L::Service>
    where
        L: Layer<T>,
        T: Transport,
        L::Service: Transport,
        <L::Service as tower::Service<Box<RawValue>>>::Future: Send,
    {
        RpcClient::new(self.builder.service(transport), is_local)
    }

    #[cfg(feature = "reqwest")]
    /// Create a new [`RpcClient`] with an HTTP transport connecting to the
    /// given URL and the configured layers.
    pub fn reqwest_http(self, url: reqwest::Url) -> RpcClient<L::Service>
    where
        L: Layer<Http<reqwest::Client>>,
        L::Service: Transport,
        <L::Service as tower::Service<Box<RawValue>>>::Future: Send,
    {
        let transport = Http::new(url);
        let is_local = transport.is_local();

        self.transport(transport, is_local)
    }
}

#[cfg(all(test, feature = "reqwest"))]
mod test {
    use crate::transports::Http;

    use super::RpcClient;

    #[test]
    fn basic_instantiation() {
        let h: RpcClient<Http<reqwest::Client>> = "http://localhost:8545".parse().unwrap();

        assert!(h.is_local());
    }
}
