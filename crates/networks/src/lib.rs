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

use alloy_json_rpc::RpcObject;

/// Captures type info for network-specific RPC requests/responses.
pub trait Network: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    /// Asserts that this trait can only be implemented on a ZST.
    const __ASSERT_ZST: () = {
        assert!(std::mem::size_of::<Self>() == 0, "Network must be a ZST");
    };

    /// The JSON body of a transaction request.
    type TransactionRequest: Transaction;

    /// The JSON body of a transaction receipt.
    type Receipt: Receipt;

    /// The JSON body of a transaction response.
    type TransactionResponse: Transaction;
}

/// Captures getters and setters common across transactions and
/// transaction-like objects across all networks.
pub trait Transaction:
    alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject + Clone + Sized + 'static
{
    /// Sets the gas price of the transaction.
    fn set_gas(&mut self, gas: alloy_primitives::U256);
}

/// Captures getters and setters common across EIP-1559 transactions across all networks
pub trait Eip1559Transaction: Transaction {}

/// Captures getters and setters common across receipts across all networks
pub trait Receipt: RpcObject + 'static {}
