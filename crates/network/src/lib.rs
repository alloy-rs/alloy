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

use alloy_eips::eip2718::Eip2718Envelope;
use alloy_json_rpc::RpcObject;
use alloy_primitives::{Address, U256};

mod transaction;
use alloy_rpc_types::TransactionList;
pub use transaction::{
    BuilderResult, NetworkSigner, TransactionBuilder, TransactionBuilderError, TxSigner,
    TxSignerSync,
};

mod ethereum;
pub use ethereum::{Ethereum, EthereumSigner};

mod any;
pub use any::AnyNetwork;

pub use alloy_eips::eip2718;

/// A transaction.
pub trait Transaction {}

/// A header.
pub trait Header {
    /// Base fee per unit of gas (if past London)
    fn base_fee_per_gas(&self) -> Option<U256>;
    /// Returns the blob fee for the next block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None.
    fn next_block_blob_fee(&self) -> Option<u128>;
}

/// A block.
pub trait Block<N: Network> {
    /// Header of the block.
    fn header(&self) -> &N::HeaderResponse;
    /// Block transactions.
    fn transactions(&self) -> &TransactionList<N::TransactionResponse>;
}

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
pub trait Network: Clone + Copy + Sized + Send + Sync + 'static {
    // -- Consensus types --

    /// The network transaction envelope type.
    type TxEnvelope: Eip2718Envelope;

    /// An enum over the various transaction types.
    type UnsignedTx;

    /// The network receipt envelope type.
    type ReceiptEnvelope: Eip2718Envelope;
    /// The network header type.
    type Header;

    // -- JSON RPC types --

    /// The JSON body of a transaction request.
    type TransactionRequest: RpcObject + TransactionBuilder<Self> + std::fmt::Debug;
    /// The JSON body of a transaction response.
    type TransactionResponse: RpcObject + Transaction;
    /// The JSON body of a transaction receipt.
    type ReceiptResponse: RpcObject + ReceiptResponse;
    /// The JSON body of a header response, as flattened into
    /// [`BlockResponse`].
    type HeaderResponse: RpcObject + Header;
    /// The JSON body of a block response
    type BlockResponse: RpcObject + Block<Self>;
}
