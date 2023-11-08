//! Alloy Transports
//!
//! ## Transport
//!
//!
//! ## PubSub services.
//!
//! ### Overview
//!
//! PubSub services, unlike regular RPC services, are long-lived and
//! bidirectional. They are used to subscribe to events on the server, and
//! receive notifications when those events occur.
//!
//! The PubSub system here consists of 3 logical parts:
//! - The **frontend** is the part of the system that the user interacts with.
//!   It exposes a simple API that allows the user to issue requests and manage
//!   subscriptions.
//! - The **service** is an intermediate layer that manages request/response
//!   mappings, subscription aliasing, and backend lifecycle events. Running
//!   [`PubSubConnect::into_service`] will spawn a long-lived service task.
//! - The **backend** is an actively running connection to the server. Users
//!   should NEVER instantiate a backend directly. Instead, they should use
//!   [`PubSubConnect::into_service`] for some connection object.
//!
//! This module provides the following:
//!
//! - [PubSubConnect]: A trait for instantiating a PubSub service by connecting
//!   to some **backend**. Implementors of this trait are responsible for
//!   the precise connection details, and for spawning the **backend** task.
//!   Users should ALWAYS call [`PubSubConnect::into_service`] to get a running
//!   service with a running backend.
//! - [`ConnectionHandle`]: A handle to a running **backend**. This type is
//!   returned by [PubSubConnect::connect], and owned by the **service**.
//!   Dropping the handle will shut down the **backend**.
//! - [`ConnectionInterface`]: The reciprocal of [ConnectionHandle]. This type
//!   is owned by the **backend**, and is used to communicate with the
//!   **service**. Dropping the interface will notify the **service** of a
//!   terminal error.
//! - [`PubSubFrontend`]: The **frontend**. A handle to a running PubSub
//!   **service**. It is used to issue requests and subscription lifecycle
//!   instructions to the **service**.
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
