mod batch;
pub use batch::BatchRequest;

mod call;
pub use call::RpcCall;

mod common;
pub use common::Authorization;

mod client;
pub use client::{ClientBuilder, RpcClient};

mod error;
pub use error::TransportError;

mod transports;
pub use transports::{BoxTransport, BoxTransportConnect, Http, Transport, TransportConnect};

pub use alloy_json_rpc::RpcResult;

pub(crate) mod utils;

pub use type_aliases::*;

#[cfg(not(target_arch = "wasm32"))]
mod type_aliases {
    use alloy_json_rpc::RpcResult;
    use serde_json::value::RawValue;

    use crate::TransportError;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = Box<serde_json::value::RawValue>, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> = std::pin::Pin<
        Box<dyn std::future::Future<Output = RpcResult<T, Box<RawValue>, E>> + Send + 'a>,
    >;
}

#[cfg(target_arch = "wasm32")]
mod type_aliases {
    use alloy_json_rpc::RpcResult;
    use serde_json::value::RawValue;

    use crate::TransportError;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = Box<serde_json::value::RawValue>, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<T, Box<RawValue>, E>> + 'a>>;
}
