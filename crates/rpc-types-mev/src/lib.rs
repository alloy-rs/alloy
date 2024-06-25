#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod eth_calls;
pub use eth_calls::*;

mod mev_calls;
pub use mev_calls::*;

// types for stats endpoint like flashbots_getUserStats and flashbots_getBundleStats
mod stats;
pub use stats::*;

mod common;
pub use common::*;

// serde helper to serialize/deserialize u256 as numeric string
mod u256_numeric_string;
