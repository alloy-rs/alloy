use std::{
    borrow::Borrow,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll},
};

use serde_json::value::RawValue;

use crate::{
    common::{Id, Request, RpcFuture, RpcOutcome},
    error::RpcResult,
    utils::to_json_raw_value,
    Connection, RpcParam, RpcResp, TransportError,
};

pub(crate) enum CallState<B, T> {
    Prepared {
        connection: B,
        method: &'static str,
        params: Box<RawValue>,
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
    SerFailure(TransportError),
}

impl<B, T> std::fmt::Debug for CallState<B, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared { method, id, .. } => f
                .debug_struct("Prepared")
                .field("method", method)
                .field("id", id)
                .finish(),
            Self::AwaitingResponse { .. } => f.debug_struct("AwaitingResponse").finish(),
            Self::SerFailure(err) => f.debug_tuple("SerFailure").field(err).finish(),
            Self::Complete => write!(f, "Complete"),
            Self::Running => write!(f, "Running"),
        }
    }
}

impl<B, T> CallState<B, T> {
    pub(crate) fn new<Params: RpcParam>(
        connection: B,
        method: &'static str,
        params: Params,
        id: Id<'static>,
    ) -> CallState<B, T> {
        let params = to_json_raw_value(&params);

        match params {
            Ok(params) => Self::Prepared {
                connection,
                method,
                params,
                id,
                _pd: PhantomData,
            },
            Err(err) => Self::SerFailure(err),
        }
    }
}

impl<B, T> CallState<B, T>
where
    B: Borrow<T>,
    T: Connection,
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

    fn poll_ser_failure(&mut self, _cx: &mut Context<'_>) -> Poll<RpcOutcome> {
        let this = std::mem::replace(self, CallState::Running);
        match this {
            CallState::SerFailure(err) => {
                *self = CallState::Complete;
                Poll::Ready(Err(err))
            }
            _ => unreachable!("called poll_ser_failure in incorrect state"),
        }
    }
}

impl<B, T> Future for CallState<B, T>
where
    B: Borrow<T> + Unpin,
    T: Connection,
{
    type Output = RpcOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state: &mut CallState<B, T> = self.get_mut();
        match state {
            CallState::Prepared { .. } => state.poll_prepared(cx),
            CallState::AwaitingResponse { .. } => state.poll_awaiting(cx),
            CallState::SerFailure(..) => state.poll_ser_failure(cx),
            _ => panic!("Polled in bad state"),
        }
    }
}

#[derive(Debug)]
pub struct RpcCall<B, T, Resp> {
    state: CallState<B, T>,
    // using `fn() -> Resp` makes this type covariant in Resp, and removes
    // drop-checking for Resp
    // c.f. https://doc.rust-lang.org/nomicon/subtyping.html#variance
    resp: PhantomData<fn() -> Resp>,
}

impl<B, T, Resp> RpcCall<B, T, Resp>
where
    Resp: RpcResp,
{
    pub fn new<Params: RpcParam>(
        connection: B,
        method: &'static str,
        params: Params,
        id: Id<'static>,
    ) -> Self {
        Self {
            state: CallState::new(connection, method, params, id),
            resp: PhantomData,
        }
    }
}

impl<B, T, Resp> Future for RpcCall<B, T, Resp>
where
    B: Borrow<T> + Unpin,
    T: Connection,
    Resp: RpcResp,
{
    type Output = RpcResult<Resp, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = Pin::new(&mut self.get_mut().state);
        let res = ready!(state.poll(cx));

        Poll::Ready(RpcResult::from(res))
    }
}

// TODO: replace these with things that aren't Box<dyn Future>
impl<'a, B, T, Resp> RpcCall<B, T, Resp>
where
    B: Borrow<T> + Unpin + 'a,
    T: Connection + 'a,
    Resp: RpcResp + 'a,
{
    /// Map the result of the future to a new type, returning a new future.
    pub fn map<U, F>(self, op: F) -> Pin<Box<dyn Future<Output = U> + 'a>>
    where
        F: FnOnce(<Self as Future>::Output) -> U + 'a,
    {
        Box::pin(async move { op(self.await) })
    }

    /// Map the result of the future to a new type, returning a new future.
    pub fn map_ok<U, F>(
        self,
        op: F,
    ) -> Pin<Box<dyn Future<Output = RpcResult<U, TransportError>> + 'a>>
    where
        F: FnOnce(Resp) -> U + 'a,
    {
        Box::pin(async move {
            let resp = self.await;
            resp.map(op)
        })
    }
}
