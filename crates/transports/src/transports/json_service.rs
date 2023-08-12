use crate::{utils::to_json_raw_value, Transport, TransportError};

use alloy_json_rpc::{JsonRpcRequest, JsonRpcResponse, RpcParam};
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use std::{future::Future, pin::Pin, task};
use tower::Service;

#[derive(Debug, Clone)]
pub struct JsonRpcService<S> {
    pub(crate) inner: S,
}

#[derive(Debug, Copy, Clone)]
pub struct JsonRpcLayer;

impl<S> tower::Layer<S> for JsonRpcLayer {
    type Service = JsonRpcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JsonRpcService { inner }
    }
}

impl<S, Param> Service<JsonRpcRequest<Param>> for JsonRpcService<S>
where
    S: Transport + Clone,
    Param: RpcParam,
{
    type Response = JsonRpcResponse;

    type Error = TransportError;

    type Future = JsonRpcFuture<S::Future, Self::Response>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: JsonRpcRequest<Param>) -> Self::Future {
        let replacement = self.inner.clone();
        let mut client = std::mem::replace(&mut self.inner, replacement);

        match to_json_raw_value(&req) {
            Ok(raw) => JsonRpcFuture {
                state: States::Pending {
                    fut: client.call(raw),
                },
                _resp: std::marker::PhantomData,
            },
            Err(e) => JsonRpcFuture {
                state: States::Errored(Some(e)),
                _resp: std::marker::PhantomData,
            },
        }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project(project = StatesProj)]
pub enum States<F> {
    Errored(Option<TransportError>),
    Pending {
        #[pin]
        fut: F,
    },
    Complete,
}

impl<F> States<F>
where
    F: Future<Output = Result<Box<RawValue>, TransportError>>,
{
    pub fn poll_errored(mut self: Pin<&mut Self>) -> task::Poll<<Self as Future>::Output> {
        let e = if let StatesProj::Errored(e) = self.as_mut().project() {
            e.take().expect("No error. This is a bug.")
        } else {
            unreachable!("Called poll_ser_error in incorrect state")
        };

        self.set(States::Complete);
        task::Poll::Ready(Err(e))
    }

    pub fn poll_pending(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<<Self as Future>::Output> {
        let StatesProj::Pending { fut } = self.as_mut().project() else {
            unreachable!("Called poll_pending in incorrect state")
        };

        fut.poll(cx)
    }
}

impl<F> Future for States<F>
where
    F: Future<Output = Result<Box<RawValue>, TransportError>>,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        match self.as_mut().project() {
            StatesProj::Errored(_) => self.poll_errored(),
            StatesProj::Pending { .. } => self.poll_pending(cx),
            StatesProj::Complete => panic!("polled after completion"),
        }
    }
}

#[pin_project::pin_project]
pub struct JsonRpcFuture<T, Resp> {
    #[pin]
    state: States<T>,
    _resp: std::marker::PhantomData<fn() -> Resp>,
}

impl<F, Resp> Future for JsonRpcFuture<F, Resp>
where
    F: Future<Output = Result<Box<RawValue>, TransportError>>,
    Resp: DeserializeOwned,
{
    type Output = Result<Resp, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let resp = task::ready!(self.project().state.poll(cx));

        task::Poll::Ready(resp.and_then(|raw| {
            serde_json::from_str(raw.get()).map_err(|err| TransportError::deser_err(err, raw.get()))
        }))
    }
}
