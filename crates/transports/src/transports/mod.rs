pub mod http;
use std::{future::Future, pin::Pin};

pub use http::Http;

use crate::TransportError;
use alloy_json_rpc::{JsonRpcRequest, JsonRpcResponse};
use tower::Service;

pub type FutureOf<S> = <S as Service<JsonRpcRequest>>::Future;
pub type BatchFutureOf<S> = <S as Service<Vec<JsonRpcRequest>>>::Future;

pub trait Transport:
    Service<
        JsonRpcRequest,
        Response = JsonRpcResponse,
        Error = TransportError,
        Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, TransportError>>>>,
    > + Service<
        Vec<JsonRpcRequest>,
        Response = Vec<JsonRpcResponse>,
        Error = TransportError,
        Future = Pin<Box<dyn Future<Output = Result<Vec<JsonRpcResponse>, TransportError>>>>,
    > + Clone
    + 'static
{
}

impl<T> Transport for T where
    T: Service<
            JsonRpcRequest,
            Response = JsonRpcResponse,
            Error = TransportError,
            Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, TransportError>>>>,
        > + Service<
            Vec<JsonRpcRequest>,
            Response = Vec<JsonRpcResponse>,
            Error = TransportError,
            Future = Pin<Box<dyn Future<Output = Result<Vec<JsonRpcResponse>, TransportError>>>>,
        > + Clone
        + 'static
{
}
