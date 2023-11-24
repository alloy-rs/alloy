#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    // TODO:
    // missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

// TODO: Add tracing.
// #[macro_use]
// extern crate tracing;

// TODO: Needed to pin version.
use protobuf as _;

mod signer;
pub use signer::TrezorSigner;

mod types;
pub use types::{DerivationType as TrezorHDPath, TrezorError};

#[doc(hidden)]
#[deprecated(note = "use `TrezorSigner` instead")]
pub type Trezor = TrezorSigner;
