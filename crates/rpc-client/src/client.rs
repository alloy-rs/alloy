use crate::{poller::PollerBuilder, BatchRequest, ClientBuilder, RpcCall};
use alloy_json_rpc::{Id, Request, RpcParam, RpcReturn};
use alloy_transport::{BoxTransport, Transport};
use alloy_transport_http::Http;
use std::{
    borrow::Cow,
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Weak,
    },
};
use tower::{layer::util::Identity, ServiceBuilder};

/// An [`RpcClient`] in a [`Weak`] reference.
pub type WeakClient<T> = Weak<RpcClientInner<T>>;

/// A borrowed [`RpcClient`].
pub type ClientRef<'a, T> = &'a RpcClientInner<T>;

/// A JSON-RPC client.
///
/// [`RpcClient`] should never be instantiated directly. Instead, use
/// [`ClientBuilder`].
///
/// [`ClientBuilder`]: crate::ClientBuilder
#[derive(Debug)]
pub struct RpcClient<T>(Arc<RpcClientInner<T>>);

impl<T> Clone for RpcClient<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl RpcClient<Identity> {
    /// Create a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder<Identity> {
        ClientBuilder { builder: ServiceBuilder::new() }
    }
}

#[cfg(feature = "reqwest")]
impl RpcClient<Http<reqwest::Client>> {
    /// Create a new [`RpcClient`] with an HTTP transport.
    pub fn new_http(url: reqwest::Url) -> Self {
        let http = Http::new(url);
        let is_local = http.guess_local();
        Self::new(http, is_local)
    }
}

impl<T> RpcClient<T> {
    /// Creates a new [`RpcClient`] with the given transport.
    pub fn new(t: T, is_local: bool) -> Self {
        Self(Arc::new(RpcClientInner::new(t, is_local)))
    }

    /// Creates a new [`RpcClient`] with the given inner client.
    pub fn from_inner(inner: RpcClientInner<T>) -> Self {
        Self(Arc::new(inner))
    }

    /// Get a reference to the client.
    pub const fn inner(&self) -> &Arc<RpcClientInner<T>> {
        &self.0
    }

    /// Convert the client into its inner type.
    pub fn into_inner(self) -> Arc<RpcClientInner<T>> {
        self.0
    }

    /// Get a [`Weak`] reference to the client.
    pub fn get_weak(&self) -> WeakClient<T> {
        Arc::downgrade(&self.0)
    }

    /// Borrow the client.
    pub fn get_ref(&self) -> ClientRef<'_, T> {
        &self.0
    }

    /// Sets the poll interval for the client in milliseconds.
    ///
    /// Note: This will only set the poll interval for the client if it is the only reference to the
    /// inner client. If the reference is held by many, then it will not update the poll interval.
    pub fn with_poll_interval(self, poll_interval: u64) -> Self {
        self.inner().set_poll_interval(poll_interval);
        self
    }
}

impl<T: Transport> RpcClient<T> {
    /// Build a poller that polls a method with the given parameters.
    ///
    /// See [`PollerBuilder`] for examples and more details.
    pub fn prepare_static_poller<Params, Resp>(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> PollerBuilder<T, Params, Resp>
    where
        T: Clone,
        Params: RpcParam + 'static,
        Resp: RpcReturn + Clone,
    {
        PollerBuilder::new(self.get_weak(), method, params)
    }
}

impl<T: Transport + Clone> RpcClient<T> {
    /// Boxes the transport.
    ///
    /// This will create a new client if this instance is not the only reference to the inner
    /// client.
    pub fn boxed(self) -> RpcClient<BoxTransport> {
        let inner = match Arc::try_unwrap(self.0) {
            Ok(inner) => inner,
            // TODO: `id` is discarded.
            Err(inner) => RpcClientInner::new(inner.transport.clone(), inner.is_local),
        };
        RpcClient::from_inner(inner.boxed())
    }
}

impl<T> RpcClient<Http<T>> {
    /// Create a new [`BatchRequest`] builder.
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<'_, Http<T>> {
        BatchRequest::new(&self.0)
    }
}

impl<T> Deref for RpcClient<T> {
    type Target = RpcClientInner<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A JSON-RPC client.
///
/// This struct manages a [`Transport`] and a request ID counter. It is used to
/// build [`RpcCall`] and [`BatchRequest`] objects. The client delegates
/// transport access to the calls.
///
/// ### Note
///
/// IDs are allocated sequentially, starting at 0. IDs are reserved via
/// [`RpcClientInner::next_id`]. Note that allocated IDs may not be used. There
/// is no guarantee that a prepared [`RpcCall`] will be sent, or that a sent
/// call will receive a response.
#[derive(Debug)]
pub struct RpcClientInner<T> {
    /// The underlying transport.
    pub(crate) transport: T,
    /// `true` if the transport is local.
    pub(crate) is_local: bool,
    /// The next request ID to use.
    pub(crate) id: AtomicU64,
    /// The poll interval for the client in milliseconds.
    pub(crate) poll_interval: AtomicU64,
}

impl<T> RpcClientInner<T> {
    /// Create a new [`RpcClient`] with the given transport.
    ///
    /// Note: Sets the poll interval to 250ms for local transports and 7s for remote transports by
    /// default.
    #[inline]
    pub const fn new(t: T, is_local: bool) -> Self {
        Self {
            transport: t,
            is_local,
            id: AtomicU64::new(0),
            poll_interval: if is_local { AtomicU64::new(250) } else { AtomicU64::new(7000) },
        }
    }

    /// Returns the default poll interval (milliseconds) for the client.
    pub fn poll_interval(&self) -> u64 {
        self.poll_interval.load(Ordering::Relaxed)
    }

    /// Set the poll interval for the client in milliseconds.
    pub fn set_poll_interval(&self, poll_interval: u64) {
        self.poll_interval.store(poll_interval, Ordering::Relaxed);
    }

    /// Returns a reference to the underlying transport.
    #[inline]
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns a mutable reference to the underlying transport.
    #[inline]
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Consumes the client and returns the underlying transport.
    #[inline]
    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Build a `JsonRpcRequest` with the given method and params.
    ///
    /// This function reserves an ID for the request, however the request is not sent.
    ///
    /// To send a request, use [`RpcClientInner::request`] and await the returned [`RpcCall`].
    #[inline]
    pub fn make_request<Params: RpcParam>(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> Request<Params> {
        Request::new(method, self.next_id(), params)
    }

    /// `true` if the client believes the transport is local.
    ///
    /// This can be used to optimize remote API usage, or to change program
    /// behavior on local endpoints. When the client is instantiated by parsing
    /// a URL or other external input, this value is set on a best-efforts
    /// basis and may be incorrect.
    #[inline]
    pub const fn is_local(&self) -> bool {
        self.is_local
    }

    /// Set the `is_local` flag.
    #[inline]
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

impl<T: Transport + Clone> RpcClientInner<T> {
    /// Prepares an [`RpcCall`].
    ///
    /// This function reserves an ID for the request, however the request is not sent.
    /// To send a request, await the returned [`RpcCall`].
    ///
    /// # Note
    ///
    /// Serialization is done lazily. It will not be performed until the call is awaited.
    /// This means that if a serializer error occurs, it will not be caught until the call is
    /// awaited.
    #[doc(alias = "prepare")]
    pub fn request<Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: Params,
    ) -> RpcCall<T, Params, Resp> {
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
    pub fn boxed(self) -> RpcClientInner<BoxTransport> {
        RpcClientInner {
            transport: self.transport.boxed(),
            is_local: self.is_local,
            id: self.id,
            poll_interval: self.poll_interval,
        }
    }
}

#[cfg(feature = "pubsub")]
mod pubsub_impl {
    use super::*;
    use alloy_pubsub::{PubSubConnect, PubSubFrontend, RawSubscription, Subscription};
    use alloy_transport::TransportResult;

    impl RpcClientInner<PubSubFrontend> {
        /// Get a [`RawSubscription`] for the given subscription ID.
        pub async fn get_raw_subscription(&self, id: alloy_primitives::U256) -> RawSubscription {
            self.transport.get_subscription(id).await.unwrap()
        }

        /// Get a [`Subscription`] for the given subscription ID.
        pub async fn get_subscription<T: serde::de::DeserializeOwned>(
            &self,
            id: alloy_primitives::U256,
        ) -> Subscription<T> {
            Subscription::from(self.get_raw_subscription(id).await)
        }
    }

    impl RpcClient<PubSubFrontend> {
        /// Connect to a transport via a [`PubSubConnect`] implementor.
        pub async fn connect_pubsub<C>(connect: C) -> TransportResult<RpcClient<PubSubFrontend>>
        where
            C: PubSubConnect,
        {
            ClientBuilder::default().pubsub(connect).await
        }

        /// Get the currently configured channel size. This is the number of items
        /// to buffer in new subscription channels. Defaults to 16. See
        /// [`tokio::sync::broadcast`] for a description of relevant
        /// behavior.
        ///
        /// [`tokio::sync::broadcast`]: https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html
        pub fn channel_size(&self) -> usize {
            self.transport.channel_size()
        }

        /// Set the channel size.
        pub fn set_channel_size(&self, size: usize) {
            self.transport.set_channel_size(size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_with_poll_interval() {
        let client = RpcClient::new_http(reqwest::Url::parse("http://localhost").unwrap())
            .with_poll_interval(5000);
        // let client = client;
        assert_eq!(client.poll_interval(), 5000);
    }
}
