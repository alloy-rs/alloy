#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

mod signer;
mod utils;
pub use signer::LedgerSigner;

mod types;
pub use types::{DerivationType as HDPath, LedgerError};

// Avoid nightly rustdoc ICEs when inlining external crate docs:
// https://github.com/paradigmxyz/solar/pull/912
#[doc(no_inline)]
pub use coins_ledger;
