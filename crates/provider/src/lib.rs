#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

// For features.
#[cfg(any(feature = "reqwest", feature = "hyper"))]
use alloy_transport_http as _;

/// Type alias for a [`RootProvider`] using the [`Http`] transport and a
/// reqwest client.
///
/// [`Http`]: alloy_transport_http::Http
#[cfg(any(test, feature = "reqwest"))]
#[deprecated(since = "0.9.0", note = "use `RootProvider` instead")]
pub type ReqwestProvider<N = alloy_network::Ethereum> = crate::RootProvider<N>;

/// Type alias for a [`RootProvider`] using the [`Http`] transport and a hyper
/// client.
///
/// [`Http`]: alloy_transport_http::Http
#[cfg(feature = "hyper")]
#[deprecated(since = "0.9.0", note = "use `RootProvider` instead")]
pub type HyperProvider<N = alloy_network::Ethereum> = crate::RootProvider<N>;

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

#[cfg(feature = "ws")]
pub use alloy_rpc_client::WsConnect;

#[cfg(feature = "ipc")]
pub use alloy_rpc_client::IpcConnect;

#[doc(no_inline)]
pub use alloy_transport::mock;
