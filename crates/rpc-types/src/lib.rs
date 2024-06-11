//! Alloy RPC type definitions.
//!
//! Provides all relevant types for the various RPC endpoints, grouped by namespace.

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub use alloy_serde as serde_helpers;

#[cfg(feature = "anvil")]
pub use alloy_rpc_types_anvil as anvil;

#[cfg(feature = "beacon")]
pub use alloy_rpc_types_beacon as beacon;

#[cfg(feature = "engine")]
pub use alloy_rpc_types_engine as engine;

#[cfg(feature = "eth")]
pub use alloy_rpc_types_eth as eth;
#[cfg(feature = "eth")]
pub use eth::*;

#[cfg(feature = "trace")]
pub use alloy_rpc_types_trace as trace;
