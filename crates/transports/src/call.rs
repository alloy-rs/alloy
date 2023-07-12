use std::{
    borrow::Borrow,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll},
};

use jsonrpsee_types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

use crate::{
    common::{Id, Request, RpcFuture, RpcOutcome},
    utils::{from_json, to_json_raw_value},
    Connection, TransportError,
};

pub(crate) enum CallState<B, T, Params> {
    Prepared {
        connection: B,
        method: &'static str,
        params: Params,
        id: Id<'static>,
        // using `fn() -> T` makes this type covariant in T, and removes
        // drop-checking for T
        // c.f. https://doc.rust-lang.org/nomicon/subtyping.html#variance
        _pd: PhantomData<fn() -> T>,
    },
    AwaitingResponse {
        fut: RpcFuture,
    },
    Complete,
    Running,
}

impl<B, T, Params> std::fmt::Debug for CallState<B, T, Params> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared { method, id, .. } => f
                .debug_struct("Prepared")
                .field("method", method)
                .field("id", id)
                .finish(),
            Self::AwaitingResponse { .. } => f.debug_struct("AwaitingResponse").finish(),
            Self::Complete => write!(f, "Complete"),
            Self::Running => write!(f, "Running"),
        }
    }
}

impl<B, T, Params> CallState<B, T, Params> {
    pub(crate) fn new(
        connection: B,
        method: &'static str,
        params: Params,
        id: Id<'static>,
    ) -> CallState<B, T, Params> {
        Self::Prepared {
            connection,
            method,
            params,
            id,
            _pd: PhantomData,
        }
    }
}

impl<B, T, Params> CallState<B, T, Params>
where
    B: Borrow<T> + Unpin,
    T: Connection + Unpin,
    Params: Serialize + Unpin,
{
    fn poll_prepared(&mut self, cx: &mut Context<'_>) -> Poll<RpcOutcome> {
        let this = std::mem::replace(self, CallState::Running);

        match this {
            CallState::Prepared {
                connection,
                method,
                params,
                id,
                ..
            } => {
                let params = to_json_raw_value(&params);
                if let Err(err) = params {
                    *self = CallState::Complete;
                    return Poll::Ready(Err(err));
                }
                let params = params.unwrap();
                let req = Request::owned(id, method, Some(params));
                let fut = connection.borrow().json_rpc_request(&req);
                *self = CallState::AwaitingResponse { fut };
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            _ => unreachable!("called poll_prepared in incorrect state"),
        }
    }

    fn poll_awaiting(&mut self, cx: &mut Context<'_>) -> Poll<RpcOutcome> {
        let this = std::mem::replace(self, CallState::Running);
        match this {
            CallState::AwaitingResponse { mut fut } => {
                if let Poll::Ready(val) = fut.as_mut().poll(cx) {
                    *self = CallState::Complete;
                    return Poll::Ready(val);
                }
                *self = CallState::AwaitingResponse { fut };
                Poll::Pending
            }
            _ => unreachable!("called poll_awaiting in incorrect state"),
        }
    }
}

impl<B, T, Params> Future for CallState<B, T, Params>
where
    B: Borrow<T> + Unpin,
    T: Connection + Unpin,
    Params: Serialize + Unpin,
{
    type Output = RpcOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.get_mut();
        match state {
            CallState::Prepared { .. } => state.poll_prepared(cx),
            CallState::AwaitingResponse { .. } => state.poll_awaiting(cx),
            _ => panic!("Polled in bad state"),
        }
    }
}

#[derive(Debug)]
pub struct RpcCall<B, T, Params, Resp> {
    state: CallState<B, T, Params>,
    // using `fn() -> Resp` makes this type covariant in Resp, and removes
    // drop-checking for Resp
    // c.f. https://doc.rust-lang.org/nomicon/subtyping.html#variance
    resp: PhantomData<fn() -> Resp>,
}

impl<B, T, Params, Resp> RpcCall<B, T, Params, Resp> {
    pub fn new(connection: B, method: &'static str, params: Params, id: Id<'static>) -> Self {
        Self {
            state: CallState::new(connection, method, params, id),
            resp: PhantomData,
        }
    }
}

impl<B, T, Params, Resp> Future for RpcCall<B, T, Params, Resp>
where
    B: Borrow<T> + Unpin,
    T: Connection + Unpin,
    Params: Serialize + Unpin,
    Resp: for<'de> Deserialize<'de> + Unpin,
{
    type Output = Result<Result<Resp, ErrorObjectOwned>, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = Pin::new(&mut self.get_mut().state);
        let res = ready!(state.poll(cx));

        match res {
            Ok(Ok(val)) => Poll::Ready(from_json(val.get()).map(Result::Ok)),
            Ok(Err(err)) => Poll::Ready(Ok(Err(err))),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
