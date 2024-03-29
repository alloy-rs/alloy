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

/* --------------------------------------- Core re-exports -------------------------------------- */

// This should generally not be used by downstream crates as we re-export everything else
// individually. It is acceptable to use this if an item has been added to `alloy-core`
// and it has not been added to the re-exports below.
#[doc(hidden)]
pub use alloy_core as core;

#[doc(inline)]
pub use self::core::primitives;
#[doc(no_inline)]
pub use primitives::{hex, uint};

#[cfg(feature = "dyn-abi")]
#[doc(inline)]
pub use self::core::dyn_abi;

#[cfg(feature = "json-abi")]
#[doc(inline)]
pub use self::core::json_abi;

#[cfg(feature = "sol-types")]
#[doc(inline)]
pub use self::core::sol_types;

// Show this re-export in docs instead of the wrapper below.
#[cfg(all(doc, feature = "sol-types"))]
#[doc(no_inline)]
pub use sol_types::sol;

/// [`sol!`](sol_types::sol!) macro wrapper to route imports to the correct crate.
///
/// See [`sol!`](sol_types::sol!) for the actual macro documentation.
#[cfg(all(not(doc), feature = "sol-types"))] // Show the actual macro in docs.
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

#[cfg(feature = "tiny-keccak")]
#[doc(inline)]
pub use self::core::tiny_keccak;

#[cfg(feature = "native-keccak")]
#[doc(inline)]
pub use self::core::native_keccak;

#[cfg(feature = "asm-keccak")]
#[doc(inline)]
pub use self::core::asm_keccak;

#[cfg(feature = "postgres")]
#[doc(inline)]
pub use self::core::postgres;

#[cfg(feature = "getrandom")]
#[doc(inline)]
pub use self::core::getrandom;

#[cfg(feature = "rand")]
#[doc(inline)]
pub use self::core::rand;

#[cfg(feature = "rlp")]
#[doc(inline)]
pub use self::core::rlp;

#[cfg(feature = "serde")]
#[doc(inline)]
pub use self::core::serde;

#[cfg(feature = "ssz")]
#[doc(inline)]
pub use self::core::ssz;

#[cfg(feature = "arbitrary")]
#[doc(inline)]
pub use self::core::arbitrary;

#[cfg(feature = "k256")]
#[doc(inline)]
pub use self::core::k256;

#[cfg(feature = "eip712")]
#[doc(inline)]
pub use self::core::eip712;

/* --------------------------------------- Main re-exports -------------------------------------- */

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

#[cfg(feature = "providers")]
pub mod providers {
    #[doc(inline)]
    pub use alloy_provider::*;
}

#[cfg(feature = "pubsub")]
pub mod pubsub {
    #[doc(inline)]
    pub use alloy_pubsub::*;
}

#[cfg(feature = "rpc")]
pub mod rpc {
    #[cfg(feature = "rpc-client")]
    #[doc(inline)]
    pub use alloy_rpc_client as client;

    #[cfg(feature = "json-rpc")]
    #[doc(inline)]
    pub use alloy_json_rpc as json_rpc;

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

#[cfg(feature = "serde")]
#[doc(inline)]
pub use alloy_serde as serde;

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

    #[cfg(feature = "signer-wallet")]
    #[doc(inline)]
    pub use alloy_signer_wallet as wallet;
}

#[cfg(feature = "transports")]
pub mod transports {
    #[doc(inline)]
    pub use alloy_transport::*;

    #[cfg(feature = "transport-http")]
    #[doc(inline)]
    pub use alloy_transport_http as http;

    #[cfg(feature = "transport-http-reqwest")]
    use reqwest as _;

    #[cfg(feature = "transport-http-hyper")]
    use hyper as _;

    #[cfg(feature = "transport-ipc")]
    #[doc(inline)]
    pub use alloy_transport_ipc as ipc;

    #[cfg(feature = "transport-ws")]
    #[doc(inline)]
    pub use alloy_transport_ws as ws;
}
