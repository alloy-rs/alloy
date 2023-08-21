use crate::{
    error::TransportError,
    transports::{JsonRpcLayer, JsonRpcService, Transport},
};

use alloy_json_rpc::{JsonRpcRequest, RpcParam, RpcResult, RpcReturn};
use core::panic;
use serde_json::value::RawValue;
use std::{future::Future, marker::PhantomData, pin::Pin, task};
use tower::{Layer, Service};

/// The states of the [`RpcCall`] future.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project(project = CallStateProj)]
enum CallState<Params, Conn>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
{
    Prepared {
        request: Option<JsonRpcRequest<Params>>,
        connection: JsonRpcService<Conn>,
    },
    AwaitingResponse {
        #[pin]
        fut: <JsonRpcService<Conn> as Service<JsonRpcRequest<Params>>>::Future,
    },
    Complete,
}

impl<Params, Conn> CallState<Params, Conn>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
{
    fn poll_prepared(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let fut = {
            let CallStateProj::Prepared {
                connection,
                request,
            } = self.as_mut().project()
            else {
                unreachable!("Called poll_prepared in incorrect state")
            };

            if let Err(e) = task::ready!(Service::<JsonRpcRequest<Params>>::poll_ready(
                connection, cx
            )) {
                self.set(CallState::Complete);
                return task::Poll::Ready(RpcResult::Err(e));
            }
            let request = request.take().expect("No request. This is a bug.");
            connection.call(request)
        };

        self.set(CallState::AwaitingResponse { fut });
        cx.waker().wake_by_ref();

        task::Poll::Pending
    }

    fn poll_awaiting(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let CallStateProj::AwaitingResponse { fut } = self.as_mut().project() else {
            unreachable!("Called poll_awaiting in incorrect state")
        };

        let res = task::ready!(fut.poll(cx));

        task::Poll::Ready(RpcResult::from(res))
    }
}

impl<Params, Conn> Future for CallState<Params, Conn>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
{
    type Output = RpcResult<Box<RawValue>, TransportError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if matches!(*self.as_mut(), CallState::Prepared { .. }) {
            return self.poll_prepared(cx);
        }

        if matches!(*self.as_mut(), CallState::AwaitingResponse { .. }) {
            return self.poll_awaiting(cx);
        }

        panic!("Polled in bad state");
    }
}

/// A prepared, but unsent, RPC call.
///
/// This is a future that will send the request when polled. It contains a
/// [`JsonRpcRequest`], a [`Transport`], and knowledge of its expected response
/// type. Upon awaiting, it will send the request and wait for the response. It
/// will then deserialize the response into the expected type.
///
/// Errors are captured in the [`RpcResult`] type. Rpc Calls will result in
/// either a successful response of the `Resp` type, an error response, or a
/// transport error.
///
/// ### Note:
///
/// Serializing the request is done lazily. The request is not serialized until
/// the future is polled.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project]
pub struct RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
{
    #[pin]
    state: CallState<Params, Conn>,
    _pd: PhantomData<fn() -> Resp>,
}

impl<Conn, Params, Resp> RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
{
    #[doc(hidden)]
    pub fn new(req: JsonRpcRequest<Params>, connection: Conn) -> Self {
        Self {
            state: CallState::Prepared {
                request: Some(req),
                connection: JsonRpcLayer.layer(connection),
            },
            _pd: PhantomData,
        }
    }

    /// Get a mutable reference to the params of the request.
    ///
    /// This is useful for modifying the params after the request has been
    /// prepared.
    pub fn params(&mut self) -> &mut Params {
        if let CallState::Prepared { request, .. } = &mut self.state {
            &mut request.as_mut().unwrap().params
        } else {
            panic!("Cannot get params after request has been sent");
        }
    }
}

impl<'a, Conn, Params, Resp> RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam + 'a,
    Resp: RpcReturn,
{
    /// Convert this future into a boxed, pinned future, erasing its type.
    pub fn boxed(
        self,
    ) -> Pin<Box<dyn Future<Output = RpcResult<Resp, TransportError>> + Send + 'a>> {
        Box::pin(self)
    }
}

impl<Conn, Params, Resp> Future for RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Conn::Future: Send,
    Params: RpcParam,
    Resp: RpcReturn,
{
    type Output = RpcResult<Resp, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let this = self.project();

        let resp = task::ready!(this.state.poll(cx));

        task::Poll::Ready(resp.deser_ok_or_else(|e, text| TransportError::deser_err(e, text)))
    }
}
