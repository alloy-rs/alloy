use alloy_json_rpc::{
    transform_response, try_deserialize_ok, Request, RequestPacket, ResponsePacket, RpcParam,
    RpcResult, RpcReturn,
};
use alloy_transport::{RpcFut, Transport, TransportError, TransportResult};
use core::panic;
use serde_json::value::RawValue;
use std::{
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, Poll::Ready},
};
use tower::Service;

/// The states of the [`RpcCall`] future.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project(project = CallStateProj)]
enum CallState<Params, Conn>
where
    Params: RpcParam,
    Conn: Transport + Clone,
{
    Prepared {
        request: Option<Request<Params>>,
        connection: Conn,
    },
    AwaitingResponse {
        #[pin]
        fut: <Conn as Service<RequestPacket>>::Future,
    },
    Complete,
}

impl<Params, Conn> Debug for CallState<Params, Conn>
where
    Params: RpcParam,
    Conn: Transport + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared { .. } => f.debug_struct("Prepared").finish(),
            Self::AwaitingResponse { .. } => f.debug_struct("AwaitingResponse").finish(),
            Self::Complete => write!(f, "Complete"),
        }
    }
}

impl<Params, Conn> CallState<Params, Conn>
where
    Conn: Transport + Clone,
    Params: RpcParam,
{
    fn poll_prepared(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        trace!("Polling prepared");
        let fut = {
            let CallStateProj::Prepared { connection, request } = self.as_mut().project() else {
                unreachable!("Called poll_prepared in incorrect state")
            };

            if let Err(e) = task::ready!(Service::<RequestPacket>::poll_ready(connection, cx)) {
                self.set(CallState::Complete);
                return Ready(RpcResult::Err(e));
            }
            let request = request.take().expect("No request. This is a bug.").serialize();

            match request {
                Ok(request) => connection.call(request.into()),
                Err(err) => {
                    self.set(CallState::Complete);
                    return Ready(RpcResult::Err(TransportError::ser_err(err)));
                }
            }
        };

        self.set(CallState::AwaitingResponse { fut });
        cx.waker().wake_by_ref();

        task::Poll::Pending
    }

    fn poll_awaiting(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        trace!("Polling awaiting");
        let CallStateProj::AwaitingResponse { fut } = self.as_mut().project() else {
            unreachable!("Called poll_awaiting in incorrect state")
        };

        match task::ready!(fut.poll(cx)) {
            Ok(ResponsePacket::Single(res)) => Ready(transform_response(res)),
            Err(e) => Ready(RpcResult::Err(e)),
            _ => panic!("received batch response from single request"),
        }
    }
}

impl<Params, Conn> Future for CallState<Params, Conn>
where
    Conn: Transport + Clone,
    Params: RpcParam,
{
    type Output = TransportResult<Box<RawValue>>;

    #[instrument(skip(self, cx))]
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
/// [`Request`], a [`Transport`], and knowledge of its expected response
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
/// the future is polled. This differs from the behavior of
/// [`crate::BatchRequest`], which serializes greedily. This is because the
/// batch request must immediately erase the `Param` type to allow batching of
/// requests with different `Param` types, while the `RpcCall` may do so lazily.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project]
#[derive(Debug)]
pub struct RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam,
{
    #[pin]
    state: CallState<Params, Conn>,
    _pd: PhantomData<fn() -> Resp>,
}

impl<Conn, Params, Resp> RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam,
{
    #[doc(hidden)]
    pub fn new(req: Request<Params>, connection: Conn) -> Self {
        Self { state: CallState::Prepared { request: Some(req), connection }, _pd: PhantomData }
    }

    /// Get a mutable reference to the params of the request.
    ///
    /// This is useful for modifying the params after the request has been
    /// prepared.
    pub fn params(&mut self) -> &mut Params {
        if let CallState::Prepared { request, .. } = &mut self.state {
            &mut request.as_mut().expect("No params in prepared. This is a bug").params
        } else {
            panic!("Cannot get params after request has been sent");
        }
    }
}

impl<'a, Conn, Params, Resp> RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam + 'a,
    Resp: RpcReturn,
{
    /// Convert this future into a boxed, pinned future, erasing its type.
    pub fn boxed(self) -> RpcFut<'a, Resp> {
        Box::pin(self)
    }
}

impl<Conn, Params, Resp> Future for RpcCall<Conn, Params, Resp>
where
    Conn: Transport + Clone,
    Params: RpcParam,
    Resp: RpcReturn,
{
    type Output = TransportResult<Resp>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        tracing::trace!(?self.state, "Polling RpcCall");
        let this = self.project();

        let result = task::ready!(this.state.poll(cx));

        Ready(try_deserialize_ok(result))
    }
}
