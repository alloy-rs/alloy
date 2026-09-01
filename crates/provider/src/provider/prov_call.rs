use alloy_json_rpc::{RequestMeta, RpcRecv, RpcSend};
use alloy_rpc_client::{RpcCall, Waiter};
use alloy_transport::TransportResult;
use futures::FutureExt;
use http::{HeaderMap, HeaderName, HeaderValue};
use pin_project::pin_project;
use serde_json::value::RawValue;
use std::{
    future::Future,
    pin::Pin,
    task::{self, Poll},
};
use tokio::sync::oneshot;

#[cfg(not(target_family = "wasm"))]
/// Boxed future type used in [`ProviderCall`] for non-wasm targets.
pub type BoxedFut<Output> = Pin<Box<dyn Future<Output = TransportResult<Output>> + Send>>;

#[cfg(target_family = "wasm")]
/// Boxed future type used in [`ProviderCall`] for wasm targets.
pub type BoxedFut<Output> = Pin<Box<dyn Future<Output = TransportResult<Output>>>>;
/// The primary future type for the [`Provider`].
///
/// This future abstracts over several potential data sources. It allows
/// providers to:
/// - produce data via an [`RpcCall`]
/// - produce data by waiting on a batched RPC [`Waiter`]
/// - proudce data via an arbitrary boxed future
/// - produce data in any synchronous way
///
/// [`Provider`]: crate::Provider
#[pin_project(project = ProviderCallProj)]
pub enum ProviderCall<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// An underlying call to an RPC server.
    RpcCall(RpcCall<Params, Resp, Output, Map>),
    /// A waiter for a batched call to a remote RPC server.
    Waiter(Waiter<Resp, Output, Map>),
    /// A boxed future.
    BoxedFuture(BoxedFut<Output>),
    /// The output, produces synchronously.
    Ready(Option<TransportResult<Output>>),
}

impl<Params, Resp, Output, Map> ProviderCall<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Instantiate a new [`ProviderCall`] from the output.
    pub const fn ready(output: TransportResult<Output>) -> Self {
        Self::Ready(Some(output))
    }

    /// True if this is an RPC call.
    pub const fn is_rpc_call(&self) -> bool {
        matches!(self, Self::RpcCall(_))
    }

    /// Fallible cast to [`RpcCall`]
    pub const fn as_rpc_call(&self) -> Option<&RpcCall<Params, Resp, Output, Map>> {
        match self {
            Self::RpcCall(call) => Some(call),
            _ => None,
        }
    }

    /// Fallible cast to mutable [`RpcCall`]
    pub const fn as_mut_rpc_call(&mut self) -> Option<&mut RpcCall<Params, Resp, Output, Map>> {
        match self {
            Self::RpcCall(call) => Some(call),
            _ => None,
        }
    }

    /// True if this is a waiter.
    pub const fn is_waiter(&self) -> bool {
        matches!(self, Self::Waiter(_))
    }

    /// Fallible cast to [`Waiter`]
    pub const fn as_waiter(&self) -> Option<&Waiter<Resp, Output, Map>> {
        match self {
            Self::Waiter(waiter) => Some(waiter),
            _ => None,
        }
    }

    /// Fallible cast to mutable [`Waiter`]
    pub const fn as_mut_waiter(&mut self) -> Option<&mut Waiter<Resp, Output, Map>> {
        match self {
            Self::Waiter(waiter) => Some(waiter),
            _ => None,
        }
    }

    /// True if this is a boxed future.
    pub const fn is_boxed_future(&self) -> bool {
        matches!(self, Self::BoxedFuture(_))
    }

    /// Fallible cast to a boxed future.
    pub const fn as_boxed_future(&self) -> Option<&BoxedFut<Output>> {
        match self {
            Self::BoxedFuture(fut) => Some(fut),
            _ => None,
        }
    }

    /// True if this is a ready value.
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Fallible cast to a ready value.
    ///
    /// # Panics
    ///
    /// Panics if the future is already complete
    pub const fn as_ready(&self) -> Option<&TransportResult<Output>> {
        match self {
            Self::Ready(Some(output)) => Some(output),
            Self::Ready(None) => panic!("tried to access ready value after taking"),
            _ => None,
        }
    }

    /// Set a function to map the response into a different type. This is
    /// useful for transforming the response into a more usable type, e.g.
    /// changing `U64` to `u64`.
    ///
    /// This function fails if the inner future is not an [`RpcCall`] or
    /// [`Waiter`].
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
    ) -> Result<ProviderCall<Params, Resp, NewOutput, NewMap>, Self>
    where
        NewMap: Fn(Resp) -> NewOutput + Clone,
    {
        match self {
            Self::RpcCall(call) => Ok(ProviderCall::RpcCall(call.map_resp(map))),
            Self::Waiter(waiter) => Ok(ProviderCall::Waiter(waiter.map_resp(map))),
            _ => Err(self),
        }
    }

    /// Maps the metadata of the underlying RPC request.
    ///
    /// This can be used with typed [`Provider`](crate::Provider) methods to
    /// attach request-scoped metadata such as HTTP headers without falling
    /// back to a raw RPC call:
    ///
    /// ```no_run
    /// # use alloy_provider::{Provider, ProviderBuilder};
    /// # use http::{HeaderMap, HeaderValue};
    /// # async fn example() -> alloy_transport::TransportResult<()> {
    /// # let provider = ProviderBuilder::new().connect("http://localhost:8545").await?;
    /// let mut headers = HeaderMap::new();
    /// headers.insert("x-api-key", HeaderValue::from_static("secret"));
    ///
    /// let call = provider.get_block_number().map_meta(|mut meta| {
    ///     meta.headers_mut().extend(headers);
    ///     meta
    /// });
    /// let Ok(call) = call else { unreachable!("typed provider method should produce an RPC call") };
    /// let block_number = call.await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This function fails if the inner future is not an [`RpcCall`], since
    /// boxed, batched, or ready calls no longer expose an individual request
    /// whose metadata can be changed.
    pub fn map_meta(self, f: impl FnOnce(RequestMeta) -> RequestMeta) -> Result<Self, Self> {
        match self {
            Self::RpcCall(call) => Ok(Self::RpcCall(call.map_meta(f))),
            _ => Err(self),
        }
    }

    /// Adds HTTP headers to the underlying RPC request.
    ///
    /// Existing values with the same header names are replaced. This function
    /// fails if the inner future is not an [`RpcCall`].
    pub fn with_headers(self, headers: HeaderMap) -> Result<Self, Self> {
        self.map_meta(|mut meta| {
            meta.headers_mut().extend(headers);
            meta
        })
    }

    /// Adds an HTTP header to the underlying RPC request.
    ///
    /// An existing value with the same header name is replaced. This function
    /// fails if the inner future is not an [`RpcCall`].
    pub fn with_header(self, name: HeaderName, value: HeaderValue) -> Result<Self, Self> {
        self.map_meta(|mut meta| {
            meta.headers_mut().insert(name, value);
            meta
        })
    }
}

impl<Params, Resp, Output, Map> ProviderCall<&Params, Resp, Output, Map>
where
    Params: RpcSend + ToOwned,
    Params::Owned: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Convert this call into one with owned params, by cloning the params.
    ///
    /// # Panics
    ///
    /// Panics if called after the request has been polled.
    pub fn into_owned_params(self) -> ProviderCall<Params::Owned, Resp, Output, Map> {
        match self {
            Self::RpcCall(call) => ProviderCall::RpcCall(call.into_owned_params()),
            _ => panic!(),
        }
    }
}

impl<Params, Resp> std::fmt::Debug for ProviderCall<Params, Resp>
where
    Params: RpcSend,
    Resp: RpcRecv,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RpcCall(call) => f.debug_tuple("RpcCall").field(call).finish(),
            Self::Waiter { .. } => f.debug_struct("Waiter").finish_non_exhaustive(),
            Self::BoxedFuture(_) => f.debug_struct("BoxedFuture").finish_non_exhaustive(),
            Self::Ready(_) => f.debug_struct("Ready").finish_non_exhaustive(),
        }
    }
}

impl<Params, Resp, Output, Map> From<RpcCall<Params, Resp, Output, Map>>
    for ProviderCall<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn from(call: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self::RpcCall(call)
    }
}

impl<Params, Resp> From<Waiter<Resp>> for ProviderCall<Params, Resp, Resp, fn(Resp) -> Resp>
where
    Params: RpcSend,
    Resp: RpcRecv,
{
    fn from(waiter: Waiter<Resp>) -> Self {
        Self::Waiter(waiter)
    }
}

impl<Params, Resp, Output, Map> From<BoxedFut<Output>> for ProviderCall<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn from(fut: BoxedFut<Output>) -> Self {
        Self::BoxedFuture(fut)
    }
}

impl<Params, Resp> From<oneshot::Receiver<TransportResult<Box<RawValue>>>>
    for ProviderCall<Params, Resp>
where
    Params: RpcSend,
    Resp: RpcRecv,
{
    fn from(rx: oneshot::Receiver<TransportResult<Box<RawValue>>>) -> Self {
        Waiter::from(rx).into()
    }
}

impl<Params, Resp, Output, Map> Future for ProviderCall<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        match self.as_mut().project() {
            ProviderCallProj::RpcCall(call) => call.poll_unpin(cx),
            ProviderCallProj::Waiter(waiter) => waiter.poll_unpin(cx),
            ProviderCallProj::BoxedFuture(fut) => fut.poll_unpin(cx),
            ProviderCallProj::Ready(output) => {
                Poll::Ready(output.take().expect("output taken twice"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc_client::{ClientBuilder, NoParams};
    use alloy_transport::mock::{Asserter, MockTransport};
    use http::HeaderValue;

    #[test]
    fn map_meta_updates_rpc_call_metadata() {
        let client = ClientBuilder::default().transport(MockTransport::new(Asserter::new()), true);
        let call: ProviderCall<NoParams, u64> = client.request_noparams("test_method").into();

        let call = call
            .map_meta(|mut meta| {
                meta.headers_mut().insert("x-api-key", HeaderValue::from_static("secret"));
                meta
            })
            .expect("call is an RPC call");

        assert_eq!(
            call.as_rpc_call().unwrap().request().meta.headers().unwrap().get("x-api-key"),
            Some(&HeaderValue::from_static("secret"))
        );
    }

    #[test]
    fn map_meta_returns_non_rpc_call() {
        let call = ProviderCall::<NoParams, u64>::ready(Ok(1));
        assert!(call.map_meta(std::convert::identity).is_err());
    }

    #[test]
    fn with_headers_updates_rpc_call_headers() {
        let client = ClientBuilder::default().transport(MockTransport::new(Asserter::new()), true);
        let call: ProviderCall<NoParams, u64> = client.request_noparams("test_method").into();
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("secret"));

        let call = call.with_headers(headers).expect("call is an RPC call");

        assert_eq!(
            call.as_rpc_call().unwrap().request().meta.headers().unwrap().get("x-api-key"),
            Some(&HeaderValue::from_static("secret"))
        );
    }

    #[test]
    fn with_header_updates_rpc_call_header() {
        let client = ClientBuilder::default().transport(MockTransport::new(Asserter::new()), true);
        let call: ProviderCall<NoParams, u64> = client.request_noparams("test_method").into();

        let call = call
            .with_header(HeaderName::from_static("x-api-key"), HeaderValue::from_static("secret"))
            .expect("call is an RPC call");

        assert_eq!(
            call.as_rpc_call().unwrap().request().meta.headers().unwrap().get("x-api-key"),
            Some(&HeaderValue::from_static("secret"))
        );
    }
}
