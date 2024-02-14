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

// Just for features.
#[cfg(feature = "transport-http-reqwest")]
use reqwest as _;

#[doc(inline)]
pub use alloy_core::*;

#[cfg(feature = "contract")]
#[doc(inline)]
pub use alloy_contract as contract;

#[cfg(feature = "consensus")]
#[doc(inline)]
pub use alloy_consensus as consensus;

#[cfg(feature = "eips")]
#[doc(inline)]
pub use alloy_eips as eips;

#[cfg(feature = "network")]
#[doc(inline)]
pub use alloy_network as network;

#[cfg(feature = "genesis")]
#[doc(inline)]
pub use alloy_genesis as genesis;

#[cfg(feature = "node-bindings")]
#[doc(inline)]
pub use alloy_node_bindings as node_bindings;

/// Interface with an Ethereum blockchain.
///
/// See [`alloy_providers`] for more details.
#[cfg(feature = "providers")]
pub mod providers {
    #[doc(inline)]
    pub use alloy_providers::*;

    // TODO: provider type aliases
    // #[cfg(feature = "provider-http")]
    // pub type HttpProvider = todo!();
    // #[cfg(feature = "provider-ws")]
    // pub type WsProvider = todo!();
    // #[cfg(feature = "provider-ipc")]
    // pub type WsProvider = todo!();
}

/// Ethereum JSON-RPC client and types.
#[cfg(feature = "rpc")]
pub mod rpc {
    #[cfg(feature = "rpc-client")]
    #[doc(inline)]
    pub use alloy_rpc_client as client;

    #[cfg(feature = "json-rpc")]
    #[doc(inline)]
    pub use alloy_json_rpc as json_rpc;

    /// Ethereum JSON-RPC type definitions.
    #[cfg(feature = "rpc-types")]
    pub mod types {
        #[cfg(feature = "rpc-types-eth")]
        #[doc(inline)]
        pub use alloy_rpc_types as eth;

        #[cfg(feature = "rpc-types-engine")]
        #[doc(inline)]
        pub use alloy_rpc_engine_types as engine;

        #[cfg(feature = "rpc-types-trace")]
        #[doc(inline)]
        pub use alloy_rpc_trace_types as trace;
    }
}

/// Ethereum signer abstraction and implementations.
///
/// See [`alloy_signer`] for more details.
#[cfg(feature = "signers")]
pub mod signers {
    #[doc(inline)]
    pub use alloy_signer::*;

    #[cfg(feature = "signer-aws")]
    #[doc(inline)]
    pub use alloy_signer_aws as aws;
    #[cfg(feature = "signer-gcp")]
    #[doc(inline)]
    pub use alloy_signer_gcp as gcp;
    #[cfg(feature = "signer-ledger")]
    #[doc(inline)]
    pub use alloy_signer_ledger as ledger;
    #[cfg(feature = "signer-trezor")]
    #[doc(inline)]
    pub use alloy_signer_trezor as trezor;
}

/// Low-level Ethereum JSON-RPC transport abstraction and implementations.
///
/// You will likely not need to use this module;
/// see the [`providers`] module for high-level usage of transports.
///
/// See [`alloy_transport`] for more details.
#[doc = "\n"] // Empty doc line `///` gets deleted by rustfmt.
#[cfg_attr(feature = "providers", doc = "[`providers`]: crate::providers")]
#[cfg_attr(
    not(feature = "providers"),
    doc = "[`providers`]: https://github.com/alloy-rs/alloy/tree/main/crates/providers"
)]
#[cfg(feature = "transports")]
pub mod transports {
    #[doc(inline)]
    pub use alloy_transport::*;

    #[cfg(feature = "transport-http")]
    #[doc(inline)]
    pub use alloy_transport_http as http;
    #[cfg(feature = "transport-ipc")]
    #[doc(inline)]
    pub use alloy_transport_ipc as ipc;
    #[cfg(feature = "transport-ws")]
    #[doc(inline)]
    pub use alloy_transport_ws as ws;
}

/// Ethereum JSON-RPC publish-subscribe tower service and type definitions.
///
/// You will likely not need to use this module;
/// see the [`providers`] module for high-level usage of pubsub.
///
/// See [`alloy_pubsub`] for more details.
#[doc = "\n"] // Empty doc line `///` gets deleted by rustfmt.
#[cfg_attr(feature = "providers", doc = "[`providers`]: crate::providers")]
#[cfg_attr(
    not(feature = "providers"),
    doc = "[`providers`]: https://github.com/alloy-rs/alloy/tree/main/crates/providers"
)]
#[cfg(feature = "pubsub")]
pub mod pubsub {
    #[doc(inline)]
    pub use alloy_pubsub::*;
}

// TODO: Enable on next alloy-core release.
/*
/// [`sol!`](sol_types::sol!) macro wrapper to route imports to the correct crate.
///
/// See [`sol!`](sol_types::sol!) for the actual macro documentation.
#[cfg(all(not(doc), feature = "sol-types"))]
#[doc(hidden)]
#[macro_export]
macro_rules! sol {
    ($($t:tt)*) => {
        $crate::sol_types::sol! {
            #![sol(alloy_sol_types = $crate::sol_types, alloy_contract = $crate::contract)]
            $($t)*
        }
    };
}
*/
