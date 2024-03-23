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

#[cfg(feature = "http")]
mod http {
    use alloy_transport_http::Http;
    use reqwest::Client as ReqwestClient;

    use crate::RootProvider;

    /// Type alias for a [`RootProvider`] using the [`Http`] transport.
    pub type HttpProvider<N> = RootProvider<N, Http<ReqwestClient>>;
}

#[cfg(feature = "http")]
pub use http::*;

#[macro_use]
extern crate tracing;

mod builder;
pub use builder::{Identity, ProviderBuilder, ProviderLayer, Stack};

pub mod layers;

mod chain;

mod heart;
pub use heart::{PendingTransaction, PendingTransactionBuilder, PendingTransactionConfig};

mod provider;
pub use provider::{FilterPollerBuilder, Provider, RootProvider};

pub mod admin;
pub mod utils;

#[doc(no_inline)]
pub use alloy_network::{self as network, Network};
