//! Alloy RPC type definitions.
//!
//! Provides all relevant types for the various RPC endpoints, grouped by namespace.

#![doc(issue_tracker_base_url = "https://github.com/alloy-rs/alloy/issues/")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod eth;
mod rpc;
mod serde_helpers;

pub use eth::*;
pub use rpc::*;
pub use serde_helpers::*;
