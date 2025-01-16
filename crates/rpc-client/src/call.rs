use alloy_json_rpc::{
    transform_response, try_deserialize_ok, Request, ResponsePacket, RpcParam, RpcReturn,
};
use alloy_transport::{BoxTransport, IntoBoxTransport, RpcFut, TransportError, TransportResult};
use core::panic;
use std::{
    fmt,
    future::{Future, IntoFuture},
    marker::PhantomData,
};
use tower::Service;

/// A prepared, but unsent, RPC call.
///
/// This is a future that will send the request when polled. It contains a
/// [`Request`], a [`BoxTransport`], and knowledge of its expected response
/// type. Upon awaiting, it will send the request and wait for the response. It
/// will then deserialize the response into the expected type.
///
/// Errors are captured in the [`RpcResult`] type. Rpc Calls will result in
/// either a successful response of the `Resp` type, an error response, or a
/// transport error.
///
/// ### Note
///
/// Serializing the request is done lazily. The request is not serialized until
/// the future is polled. This differs from the behavior of
/// [`crate::BatchRequest`], which serializes greedily. This is because the
/// batch request must immediately erase the `Param` type to allow batching of
/// requests with different `Param` types, while the `RpcCall` may do so lazily.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Clone)]
pub struct RpcCall<Params, Resp, Output = Resp, Map = fn(Resp) -> Output> {
    request: Request<Params>,
    connection: BoxTransport,
    map: Map,
    _pd: core::marker::PhantomData<fn() -> (Resp, Output)>,
}

impl<Params, Resp, Output, Map> fmt::Debug for RpcCall<Params, Resp, Output, Map>
where
    Params: RpcParam,
    Map: FnOnce(Resp) -> Output,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcCall").finish_non_exhaustive()
    }
}

impl<Params, Resp> RpcCall<Params, Resp>
where
    Params: RpcParam,
{
    #[doc(hidden)]
    pub fn new(request: Request<Params>, connection: impl IntoBoxTransport) -> Self {
        Self {
            request,
            connection: connection.into_box_transport(),
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<Params, Resp, Output, Map> RpcCall<Params, Resp, Output, Map>
where
    Params: RpcParam,
    Map: FnOnce(Resp) -> Output,
{
    /// Map the response to a different type. This is usable for converting
    /// the response to a more usable type, e.g. changing `U64` to `u64`.
    ///
    /// ## Note
    ///
    /// Carefully review the rust documentation on [fn pointers] before passing
    /// them to this function. Unless the pointer is specifically coerced to a
    /// `fn(_) -> _`, the `NewMap` will be inferred as that function's unique
    /// type. This can lead to confusing error messages.
    ///
    /// [fn pointers]: https://doc.rust-lang.org/std/primitive.fn.html#creating-function-pointers
    pub fn map_resp<NewOutput, NewMap>(
        self,
        map: NewMap,
    ) -> RpcCall<Params, Resp, NewOutput, NewMap>
    where
        NewMap: FnOnce(Resp) -> NewOutput,
    {
        RpcCall { request: self.request, connection: self.connection, map, _pd: PhantomData }
    }

    /// Returns `true` if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.request().meta.is_subscription()
    }

    /// Set the request to be a non-standard subscription (i.e. not
    /// "eth_subscribe").
    pub fn set_is_subscription(&mut self) {
        self.request_mut().meta.set_is_subscription();
    }

    /// Set the subscription status of the request.
    pub fn set_subscription_status(&mut self, status: bool) {
        self.request_mut().meta.set_subscription_status(status);
    }

    /// Get a mutable reference to the params of the request.
    ///
    /// This is useful for modifying the params after the request has been
    /// prepared.
    pub fn params(&mut self) -> &mut Params {
        &mut self.request_mut().params
    }

    /// Returns a reference to the request.
    pub fn request(&self) -> &Request<Params> {
        &self.request
    }

    /// Returns a mutable reference to the request.
    pub fn request_mut(&mut self) -> &mut Request<Params> {
        &mut self.request
    }

    /// Map the params of the request into a new type.
    pub fn map_params<NewParams: RpcParam>(
        self,
        map: impl FnOnce(Params) -> NewParams,
    ) -> RpcCall<NewParams, Resp, Output, Map> {
        RpcCall {
            request: self.request.map_params(map),
            connection: self.connection,
            map: self.map,
            _pd: PhantomData,
        }
    }
}

impl<Params, Resp, Output, Map> RpcCall<&Params, Resp, Output, Map>
where
    Params: RpcParam + ToOwned,
    Params::Owned: RpcParam,
    Map: FnOnce(Resp) -> Output,
{
    /// Convert this call into one with owned params, by cloning the params.
    ///
    /// # Panics
    ///
    /// Panics if called after the request has been polled.
    pub fn into_owned_params(self) -> RpcCall<Params::Owned, Resp, Output, Map> {
        RpcCall {
            request: self.request.into_owned_params(),
            connection: self.connection,
            map: self.map,
            _pd: PhantomData,
        }
    }
}

impl<'a, Params, Resp, Output, Map> RpcCall<Params, Resp, Output, Map>
where
    Params: RpcParam + 'a,
    Resp: RpcReturn,
    Output: 'a,
    Map: FnOnce(Resp) -> Output + Send + 'a,
{
    /// Convert this future into a boxed, pinned future, erasing its type.
    pub fn boxed(self) -> RpcFut<'a, Output> {
        self.into_future()
    }

    async fn do_call(self) -> TransportResult<Output> {
        let Self { request, mut connection, map, _pd: PhantomData } = self;
        std::future::poll_fn(|cx| connection.poll_ready(cx)).await?;
        let serialized_request = request.serialize().map_err(TransportError::ser_err)?;
        let response_packet = connection.call(serialized_request.into()).await?;
        let ResponsePacket::Single(response) = response_packet else {
            panic!("received batch response from single request")
        };
        try_deserialize_ok(transform_response(response)).map(map)
    }
}

impl<'a, Params, Resp, Output, Map> IntoFuture for RpcCall<Params, Resp, Output, Map>
where
    Params: RpcParam + 'a,
    Resp: RpcReturn,
    Output: 'a,
    Map: FnOnce(Resp) -> Output + Send + 'a,
{
    type IntoFuture = RpcFut<'a, Output>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.do_call())
    }
}
