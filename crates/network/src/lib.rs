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

mod eip2718;
pub use eip2718::{Decodable2718, Eip2718Envelope, Eip2718Error, Encodable2718};

mod sealed;
pub use sealed::{Sealable, Sealed};

mod transaction;
pub use transaction::{Eip1559Transaction, Signed, Transaction};

mod receipt;
pub use receipt::Receipt;

use alloy_json_rpc::RpcObject;
use alloy_primitives::B256;

/// A list of transactions, either hydrated or hashes.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TransactionList<T> {
    /// Hashes only.
    Hashes(Vec<B256>),
    /// Hydrated tx objects.
    Hydrated(Vec<T>),
}

/// A block response
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BlockResponse<N: Network> {
    #[serde(flatten)]
    header: N::HeaderResponse,
    transactions: TransactionList<N::TransactionResponse>,
}

/// Captures type info for network-specific RPC requests/responses.
pub trait Network: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    /// Asserts that this trait can only be implemented on a ZST.
    const __ASSERT_ZST: () = {
        assert!(std::mem::size_of::<Self>() == 0, "Network must be a ZST");
    };

    /// The JSON body of a transaction request.
    type TransactionRequest: RpcObject + Transaction; // + TransactionBuilder

    /// The JSON body of a transaction response.
    type TransactionResponse: RpcObject;

    /// The JSON body of a transaction receipt.
    type ReceiptResponse: RpcObject;

    /// The JSON body of a header response, as flattened into
    /// [`BlockResponse`].
    type HeaderResponse: RpcObject;
}
