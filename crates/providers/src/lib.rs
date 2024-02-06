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

mod builder;
pub use builder::{ProviderBuilder, ProviderLayer, Stack};

mod chain;

mod heart;

pub mod new;
pub use new::{ProviderRef, RootProvider, WeakProvider, Provider};

pub mod utils;

// TODO: remove
pub mod tmp;
