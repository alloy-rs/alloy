pub mod http;
use std::{future::Future, pin::Pin};

use alloy_json_rpc::JsonRpcResponse;
pub use http::Http;

use crate::TransportError;

pub type TransportFuture =
    Pin<Box<dyn Future<Output = Result<JsonRpcResponse, TransportError>> + Send>>;

pub type BatchTransportFuture =
    Pin<Box<dyn Future<Output = Result<Vec<JsonRpcResponse>, TransportError>> + Send>>;
