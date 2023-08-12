use crate::{
    error::TransportError,
    transports::{JsonRpcLayer, JsonRpcService, Transport},
};

use alloy_json_rpc::{JsonRpcRequest, RpcParam, RpcResult, RpcReturn};
use serde_json::value::RawValue;
use std::{future::Future, marker::PhantomData, pin::Pin, task};
use tower::{Layer, Service};

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
    pub fn new(req: JsonRpcRequest<Params>, connection: Conn) -> Self {
        Self {
            state: CallState::Prepared {
                request: Some(req),
                connection: JsonRpcLayer.layer(connection),
            },
            _pd: PhantomData,
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
    pub fn box_pin(
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
