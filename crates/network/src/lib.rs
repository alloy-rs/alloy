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

use alloy_consensus::TxReceipt;
use alloy_eips::eip2718::{Eip2718Envelope, Eip2718Error};
use alloy_json_rpc::RpcObject;
use alloy_primitives::Address;
use core::fmt::{Debug, Display};

mod transaction;
pub use transaction::{
    BuildResult, NetworkSigner, TransactionBuilder, TransactionBuilderError, TxSigner,
    TxSignerSync, Unbuilt,
};

mod ethereum;
pub use ethereum::{Ethereum, EthereumSigner};

mod any;
pub use any::AnyNetwork;

pub use alloy_eips::eip2718;

/// A receipt response.
///
/// This is distinct from [`TxReceipt`], since this is for JSON-RPC receipts.
///
/// [`TxReceipt`]: alloy_consensus::TxReceipt
pub trait ReceiptResponse {
    /// Address of the created contract, or `None` if the transaction was not a deployment.
    fn contract_address(&self) -> Option<Address>;
}

/// Captures type info for network-specific RPC requests/responses.
///
/// Networks are only containers for types, so it is recommended to use ZSTs for their definition.
// todo: block responses are ethereum only, so we need to include this in here too, or make `Block`
// generic over tx/header type
pub trait Network: Debug + Clone + Copy + Sized + Send + Sync + 'static {
    // -- Consensus types --

    /// The network transaction type enum.
    ///
    /// This should be a simple `#[repr(u8)]` enum, and as such has strict type
    /// bounds for better use in error messages, assertions etc.
    type TxType: Into<u8>
        + PartialEq
        + Eq
        + TryFrom<u8, Error = Eip2718Error>
        + Debug
        + Display
        + Clone
        + Copy
        + Send
        + Sync
        + 'static;

    /// The network transaction envelope type.
    type TxEnvelope: Eip2718Envelope + Debug;

    /// An enum over the various transaction types.
    type UnsignedTx: From<Self::TxEnvelope>;

    /// The network receipt envelope type.
    type ReceiptEnvelope: Eip2718Envelope + TxReceipt;

    /// The network header type.
    type Header;

    // -- JSON RPC types --

    /// The JSON body of a transaction request.
    type TransactionRequest: RpcObject
        + TransactionBuilder<Self>
        + Debug
        + From<Self::TxEnvelope>
        + From<Self::UnsignedTx>;

    /// The JSON body of a transaction response.
    type TransactionResponse: RpcObject;

    /// The JSON body of a transaction receipt.
    type ReceiptResponse: RpcObject + ReceiptResponse;

    /// The JSON body of a header response.
    type HeaderResponse: RpcObject;
}
