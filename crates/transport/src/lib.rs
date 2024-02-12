#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod boxed;
pub use boxed::BoxTransport;

mod connect;
pub use connect::{BoxTransportConnect, TransportConnect};

mod common;
pub use common::Authorization;

mod error;
#[doc(hidden)]
pub use error::TransportErrorKind;
pub use error::{TransportError, TransportResult};

mod r#trait;
pub use r#trait::Transport;

pub use alloy_json_rpc::{RpcError, RpcResult};

/// Misc. utilities for building transports.
pub mod utils;

pub use type_aliases::*;

#[cfg(not(target_arch = "wasm32"))]
mod type_aliases {
    use crate::{TransportError, TransportResult};
    use alloy_json_rpc::ResponsePacket;

    /// Pin-boxed future.
    pub type Pbf<'a, T, E> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = ResponsePacket, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T> =
        std::pin::Pin<Box<dyn std::future::Future<Output = TransportResult<T>> + Send + 'a>>;
}

#[cfg(target_arch = "wasm32")]
mod type_aliases {
    use crate::{TransportError, TransportResult};
    use alloy_json_rpc::ResponsePacket;

    /// Pin-boxed future.
    pub type Pbf<'a, T, E> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for Transport-level requests.
    pub type TransportFut<'a, T = ResponsePacket, E = TransportError> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a>>;

    /// Future for RPC-level requests.
    pub type RpcFut<'a, T> =
        std::pin::Pin<Box<dyn std::future::Future<Output = TransportResult<T>> + 'a>>;
}
