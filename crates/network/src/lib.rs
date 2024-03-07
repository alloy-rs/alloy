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
use alloy_primitives::B256;

mod transaction;
pub use transaction::{
    BuilderResult, NetworkSigner, TransactionBuilder, TransactionBuilderError, TxSigner,
    TxSignerSync,
};

mod receipt;
pub use receipt::Receipt;

pub use alloy_eips::eip2718;

mod ethereum;
pub use ethereum::{Ethereum, EthereumSigner};

/// A list of transactions, either hydrated or hashes.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TransactionList<T> {
    /// Hashes only.
    Hashes(Vec<B256>),
    /// Hydrated tx objects.
    Hydrated(Vec<T>),
    /// Special case for uncle response
    Uncled,
}

/// A block response
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BlockResponse<N: Network> {
    #[serde(flatten)]
    header: N::HeaderResponse,
    transactions: TransactionList<N::TransactionResponse>,
}

/// Captures type info for network-specific RPC requests/responses.
// todo: block responses are ethereum only, so we need to include this in here too, or make `Block`
// generic over tx/header type
pub trait Network: Clone + Copy + Sized + Send + Sync + 'static {
    #[doc(hidden)]
    /// Asserts that this trait can only be implemented on a ZST.
    const __ASSERT_ZST: () = {
        assert!(std::mem::size_of::<Self>() == 0, "Network must be a ZST");
    };

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
    type TransactionResponse: RpcObject;
    /// The JSON body of a transaction receipt.
    type ReceiptResponse: RpcObject;
    /// The JSON body of a header response, as flattened into
    /// [`BlockResponse`].
    type HeaderResponse: RpcObject;
}
