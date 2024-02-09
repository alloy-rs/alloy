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

pub mod constants;

mod header;
use alloy_network::Network;
pub use header::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod receipt;
pub use receipt::{Receipt, ReceiptEnvelope, ReceiptWithBloom};

mod transaction;
pub use transaction::{
    EthereumTxBuilder, TxEip1559, TxEip2930, TxEip4844, TxEnvelope, TxLegacy, TxType,
    TypedTransaction,
};

pub use alloy_network::TxKind;

struct Ethereum;

impl Network for Ethereum {
    type TxEnvelope = TxEnvelope;

    type UnsignedTx = TypedTransaction;

    type ReceiptEnvelope = ReceiptEnvelope;

    type Header = Header;

    type TransactionBuilder = EthereumTxBuilder;

    type TransactionRequest = alloy_rpc_types::Transaction;

    type TransactionResponse = alloy_rpc_types::Transaction;

    type ReceiptResponse = alloy_rpc_types::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types::Header;
}
