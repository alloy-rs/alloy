mod http;
pub use http::Http;

mod json_service;
pub(crate) use json_service::{JsonRpcLayer, JsonRpcService};

use serde_json::value::RawValue;
use std::{future::Future, pin::Pin};
use tower::Service;

use crate::TransportError;

pub trait Transport:
    Service<
        Box<RawValue>,
        Response = Box<RawValue>,
        Error = TransportError,
        Future = Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send>>,
    > + Send
    + Sync
    + 'static
{
    fn boxed(self) -> BoxTransport
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        BoxTransport {
            inner: Box::new(self),
        }
    }
}

impl<T> Transport for T where
    T: Service<
            Box<RawValue>,
            Response = Box<RawValue>,
            Error = TransportError,
            Future = Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send>>,
        > + Send
        + Sync
        + 'static
{
}

pub struct BoxTransport {
    inner: Box<dyn CloneTransport + Send + Sync>,
}

impl Clone for BoxTransport {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

trait CloneTransport: Transport {
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync>;
}

impl<T> CloneTransport for T
where
    T: Transport + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn CloneTransport + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Service<Box<RawValue>> for BoxTransport {
    type Response = Box<RawValue>;

    type Error = TransportError;

    type Future = Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.inner.call(req)
    }
}

/// checks trait + send + sync + 'static
fn __compile_check() {
    fn inner<T: CloneTransport>() {
        todo!()
    }
    inner::<BoxTransport>();
}
