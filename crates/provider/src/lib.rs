#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

// For features.
#[cfg(any(feature = "reqwest", feature = "hyper"))]
use alloy_transport_http as _;

#[macro_use]
extern crate tracing;

mod builder;
pub use builder::*;

mod blocks;

pub mod ext;

pub mod fillers;

mod heart;
pub use heart::*;

pub mod layers;

mod provider;
pub use provider::*;

pub mod utils;

#[doc(no_inline)]
pub use alloy_network::{self as network, Network};

pub use alloy_transport as transport;

pub use alloy_rpc_client::ConnectionConfig;

#[cfg(feature = "ws")]
pub use alloy_rpc_client::WsConnect;

#[cfg(all(feature = "ws", not(target_family = "wasm")))]
pub use alloy_rpc_client::WebSocketConfig;

#[cfg(feature = "ipc")]
pub use alloy_rpc_client::IpcConnect;

#[doc(no_inline)]
pub use alloy_transport::mock;
