mod http;
pub use http::Http;

mod json_service;
pub use json_service::{JsonRpcFuture, JsonRpcLayer, JsonRpcService};

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
