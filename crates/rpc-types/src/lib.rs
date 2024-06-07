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

#[cfg(feature = "rpc-types-anvil")]
pub use alloy_rpc_types_anvil as anvil;

#[cfg(feature = "rpc-types-beacon")]
pub use alloy_rpc_types_beacon as beacon;

#[cfg(feature = "rpc-types-engine")]
pub use alloy_rpc_types_engine as engine;

#[cfg(feature = "rpc-types-eth")]
pub use alloy_rpc_types_eth::*;

#[cfg(feature = "rpc-types-trace")]
pub use alloy_rpc_types_trace as trace;
