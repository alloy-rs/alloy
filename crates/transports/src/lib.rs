mod call;
pub use call::RpcCall;

mod common;
pub use common::Authorization;

/// [`RpcClient`] and [`ClientBuilder`].
pub mod client;
pub use client::{ClientBuilder, RpcClient};

mod error;
pub use error::TransportError;

mod batch;
pub use batch::BatchRequest;

mod transports;
pub use transports::{BoxTransport, Http, Transport};

mod pubsub;
pub use pubsub::{BoxPubSub, PubSub, PubSubConnect};

pub(crate) mod utils;

pub use alloy_json_rpc::RpcResult;
pub use type_aliases::*;

#[cfg(not(target_arch = "wasm32"))]
mod type_aliases {
    use alloy_json_rpc::RpcResult;

    use crate::TransportError;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = Box<serde_json::value::RawValue>, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<T, E>> + Send + 'a>>;
}

#[cfg(target_arch = "wasm32")]
mod type_aliases {
    use alloy_json_rpc::RpcResult;

    use crate::TransportError;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = Box<serde_json::value::RawValue>, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<T, E>> + 'a>>;
}
