//! Alloy RPC type definitions.
//!
//! Provides all relevant types for the various RPC endpoints, grouped by namespace.

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

mod eth;

pub use alloy_serde as serde_helpers;
pub use eth::*;

mod transaction_list;
pub use transaction_list::TransactionList;

mod with_other;
pub use with_other::WithOtherFields;
