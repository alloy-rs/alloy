#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod account;
pub use account::Account;

mod block;
pub use block::{Block, BlockBody};

pub mod constants;
pub use constants::{EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod encodable_signature;
pub use encodable_signature::EncodableSignature;

mod header;
pub use header::{BlockHeader, Header};

mod receipt;
pub use receipt::{
    AnyReceiptEnvelope, Eip658Value, Receipt, ReceiptEnvelope, ReceiptWithBloom, Receipts,
    TxReceipt,
};

pub mod transaction;
#[cfg(feature = "kzg")]
pub use transaction::BlobTransactionValidationError;
pub use transaction::{
    SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEip7702, TxEnvelope, TxLegacy, TxType, TypedTransaction,
};

pub use alloy_eips::eip4844::{
    builder::{SidecarBuilder, SidecarCoder, SimpleCoder},
    utils, Blob, BlobTransactionSidecar, Bytes48,
};

#[cfg(feature = "kzg")]
pub use alloy_eips::eip4844::env_settings::EnvKzgSettings;

pub use alloy_primitives::{Sealable, Sealed};

mod signed;
pub use signed::Signed;

/// Bincode-compatible serde implementations for consensus types.
///
/// `bincode` crate doesn't work well with optionally serializable serde fields, but some of the
/// consensus types require optional serialization for RPC compatibility. This module makes so that
/// all fields are serialized.
///
/// Read more: <https://github.com/bincode-org/bincode/issues/326>
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub mod serde_bincode_compat {
    pub use super::{
        header::serde_bincode_compat::*,
        transaction::{serde_bincode_compat as transaction, serde_bincode_compat::*},
    };
}
