mod http;
pub use http::Http;

mod json_rpc;
pub use json_rpc::{JsonRpcFuture, JsonRpcLayer, JsonRpcService};

use serde_json::value::RawValue;
use std::{future::Future, pin::Pin};
use tower::{util::BoxCloneService, Service};

use crate::TransportError;

pub trait Transport:
    Service<
        Box<RawValue>,
        Response = Box<RawValue>,
        Error = TransportError,
        Future = Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send>>,
    > + Clone
    + Send
    + 'static
{
}

impl<T> Transport for T where
    T: Service<
            Box<RawValue>,
            Response = Box<RawValue>,
            Error = TransportError,
            Future = Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send>>,
        > + Clone
        + Send
        + 'static
{
}

fn __compile_check() -> impl Transport {
    let a: BoxCloneService<Box<RawValue>, Box<RawValue>, TransportError> = todo!();
    a
}
