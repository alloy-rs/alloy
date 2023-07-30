use crate::{error::TransportError, transports::FutureOf};

use alloy_json_rpc::{JsonRpcRequest, JsonRpcResponse, RpcParam, RpcResult, RpcReturn};
use serde_json::value::RawValue;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, ready},
};
use tower::Service;

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project(project = CallStateProj)]
enum CallState<Conn>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
{
    Prepared {
        request: Option<JsonRpcRequest>,
        connection: Conn,
    },
    AwaitingResponse {
        #[pin]
        fut: FutureOf<Conn>,
    },
    Complete,
    SerError(Option<TransportError>),
}

impl<Conn> CallState<Conn>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
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

            if let Err(e) = task::ready!(connection.poll_ready(cx)) {
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

        let res = ready!(fut.poll(cx));

        task::Poll::Ready(RpcResult::from(res))
    }

    fn poll_ser_error(
        mut self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let e = if let CallStateProj::SerError(e) = self.as_mut().project() {
            e.take().expect("No error. This is a bug.")
        } else {
            unreachable!("Called poll_ser_error in incorrect state")
        };

        self.set(CallState::Complete);
        task::Poll::Ready(RpcResult::Err(e))
    }
}

impl<Conn> Future for CallState<Conn>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
{
    type Output = RpcResult<Box<RawValue>, TransportError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if matches!(*self.as_mut(), CallState::Prepared { .. }) {
            return self.poll_prepared(cx);
        }

        if matches!(*self.as_mut(), CallState::AwaitingResponse { .. }) {
            return self.poll_awaiting(cx);
        }

        if matches!(*self.as_mut(), CallState::SerError(_)) {
            return self.poll_ser_error(cx);
        }

        panic!("Polled in bad state");
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project]
pub struct RpcCall<Conn, Params, Resp>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
    Params: RpcParam,
{
    #[pin]
    state: CallState<Conn>,
    _pd: PhantomData<fn() -> (Params, Resp)>,
}

impl<Conn, Params, Resp> RpcCall<Conn, Params, Resp>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
    Params: RpcParam,
{
    pub fn new(request: Result<JsonRpcRequest, TransportError>, connection: Conn) -> Self {
        let state = match request {
            Ok(req) => CallState::Prepared {
                request: Some(req),
                connection,
            },
            Err(e) => CallState::SerError(Some(e)),
        };

        Self {
            state,
            _pd: PhantomData,
        }
    }
}

impl<Conn, Params, Resp> Future for RpcCall<Conn, Params, Resp>
where
    Conn: Service<JsonRpcRequest, Response = JsonRpcResponse, Error = TransportError>,
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
