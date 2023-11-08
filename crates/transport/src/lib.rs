//! Alloy Transports
//!
//! ## Transport
//!
//!
mod boxed;
pub use boxed::BoxTransport;

mod connect;
pub use connect::{BoxTransportConnect, TransportConnect};

mod common;
pub use common::Authorization;

mod error;
pub use error::TransportError;

mod r#trait;
pub use r#trait::Transport;

pub use alloy_json_rpc::RpcResult;

pub mod utils;

pub use type_aliases::*;

#[cfg(not(target_arch = "wasm32"))]
mod type_aliases {
    use alloy_json_rpc::{ResponsePacket, RpcResult};
    use serde_json::value::RawValue;

    use crate::TransportError;

    pub type Pbf<'a, T, E> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = ResponsePacket, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> = std::pin::Pin<
        Box<dyn std::future::Future<Output = RpcResult<T, Box<RawValue>, E>> + Send + 'a>,
    >;
}

#[cfg(target_arch = "wasm32")]
mod type_aliases {
    use alloy_json_rpc::{ResponsePacket, RpcResult};
    use serde_json::value::RawValue;

    use crate::TransportError;

    pub type Pbf<'a, T, E> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = ResponsePacket, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<T, Box<RawValue>, E>> + 'a>>;
}
