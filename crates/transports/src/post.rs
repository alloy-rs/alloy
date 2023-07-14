use std::{
    borrow::Borrow,
    fmt,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::{error::RpcResult, Connection, RpcResp, TransportError};

use super::RpcCall;

impl<'a, B, T, Resp> RpcCall<B, T, Resp> {
    pub fn post_process<F>(self, f: F) -> CallWithPost<'a, 'static, B, T, Resp>
    where
        F: FnOnce(Resp) -> Resp + 'a,
    {
        CallWithPost {
            inner: self,
            post: Some(Box::new(f)),
            transform: Box::new(|x| x),
        }
    }
}

pub struct CallWithPost<'a, 'b, B, T, Resp, U = Resp> {
    pub(crate) inner: RpcCall<B, T, Resp>,
    pub(crate) post: Option<Box<dyn FnOnce(Resp) -> Resp + 'a>>,
    pub(crate) transform: Box<dyn FnOnce(Resp) -> U + 'b>,
}

impl<B, T, Resp> From<RpcCall<B, T, Resp>> for CallWithPost<'_, '_, B, T, Resp> {
    fn from(value: RpcCall<B, T, Resp>) -> Self {
        Self {
            inner: value,
            post: None,
            transform: Box::new(|x| x),
        }
    }
}

impl<'a, B, T, Resp> std::fmt::Debug for CallWithPost<'a, '_, B, T, Resp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallWithPost").finish()
    }
}

impl<B, T, Resp, U> Future for CallWithPost<'_, '_, B, T, Resp, U>
where
    B: Borrow<T> + Unpin,
    T: Connection,
    Resp: RpcResp,
{
    type Output = RpcResult<U, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut self.get_mut();
        let inner = Pin::new(&mut this.inner);

        let mut output = ready!(inner.poll(cx));

        if let Some(post) = this.post.take() {
            output = output.map(post);
        }

        // SAFETY: this future may never be polled again. Leaving the closure
        // in a bad state is not a problem.
        let transform = std::mem::replace(&mut this.transform, Box::new(|_| panic!()));

        Poll::Ready(output.map(transform))
    }
}

impl<'process, 'transform, B, T, Resp> CallWithPost<'process, 'transform, B, T, Resp>
where
    B: Borrow<T> + Unpin,
    T: Connection,
    Resp: RpcResp,
{
    pub fn and<'b, 'c, New>(mut self, new: New) -> CallWithPost<'c, 'transform, B, T, Resp>
    where
        Resp: 'c,
        New: FnOnce(Resp) -> Resp + Unpin + 'b,
        'process: 'c,
        'b: 'c,
    {
        let prev = self.post.take().expect("modified after complete");

        let post = compose(prev, new);

        CallWithPost {
            inner: self.inner,
            post: Some(Box::new(post)),
            transform: self.transform,
        }
    }

    pub fn instead<'b, New>(self, new: New) -> CallWithPost<'b, 'transform, B, T, Resp>
    where
        New: FnOnce(Resp) -> Resp + Unpin + 'b,
    {
        CallWithPost {
            inner: self.inner,
            post: Some(Box::new(new)),
            transform: self.transform,
        }
    }

    pub fn and_transform<'b, 'c, Transform, U>(
        self,
        t: Transform,
    ) -> CallWithPost<'process, 'c, B, T, Resp, U>
    where
        'transform: 'c,
        'b: 'c,
        Resp: 'c,
        Transform: FnOnce(Resp) -> U + Unpin + 'b,
    {
        let prev = self.transform;

        let transform = compose_transform(prev, t);

        CallWithPost {
            inner: self.inner,
            post: self.post,
            transform: Box::new(transform),
        }
    }
}

pub(crate) fn compose<'a, 'b, 'c, Fn1, Fn2, T>(f1: Fn1, f2: Fn2) -> impl FnOnce(T) -> T + 'c
where
    Fn1: FnOnce(T) -> T + 'a,
    Fn2: FnOnce(T) -> T + 'b,
    'a: 'c,
    'b: 'c,
{
    move |x| f2(f1(x))
}

pub(crate) fn compose_transform<'a, 'b, 'c, Fn1, Fn2, T1, T2, T3>(
    f1: Fn1,
    f2: Fn2,
) -> impl FnOnce(T1) -> T3 + 'c
where
    Fn1: FnOnce(T1) -> T2 + 'a,
    Fn2: FnOnce(T2) -> T3 + 'b,
    'a: 'c,
    'b: 'c,
{
    move |x| f2(f1(x))
}
