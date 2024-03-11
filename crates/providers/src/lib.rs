#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    // TODO:
    // missing_copy_implementations,
    // missing_debug_implementations,
    // missing_docs,
    unreachable_pub,
    // clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use alloy_transport_http::Http;
use reqwest::Client as ReqwestClient;

/// Type alias for a [`RootProvider`] using the [`Http`] transport.
pub type HttpProvider<N> = RootProvider<N, Http<ReqwestClient>>;

#[macro_use]
extern crate tracing;

mod builder;
pub use builder::{Identity, ProviderBuilder, ProviderLayer, Stack};

mod signer;
pub use signer::{SignerLayer, SignerProvider};

mod chain;

mod heart;
pub use heart::{PendingTransaction, PendingTransactionConfigInner};

pub mod new;

#[doc(inline)]
pub use new::{AnvilProvider, Provider, ProviderRef, RawProvider, RootProvider, WeakProvider};

pub mod utils;

#[doc(no_inline)]
pub use alloy_network::{self as network, Network};
