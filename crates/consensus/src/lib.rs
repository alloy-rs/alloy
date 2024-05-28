#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod account;
pub use account::Account;

pub mod constants;

mod header;
pub use header::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod receipt;
pub use receipt::{AnyReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom, TxReceipt};

mod request;
pub use request::Request;

pub mod transaction;
#[cfg(feature = "kzg")]
pub use transaction::BlobTransactionValidationError;
pub use transaction::{
    SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEnvelope, TxLegacy, TxType, TypedTransaction,
};

pub use alloy_eips::eip4844::{
    builder::{SidecarBuilder, SidecarCoder, SimpleCoder},
    utils, Blob, BlobTransactionSidecar, Bytes48,
};

#[cfg(feature = "kzg")]
pub use alloy_eips::eip4844::env_settings::EnvKzgSettings;

mod sealed;
pub use sealed::{Sealable, Sealed};

mod signed;
pub use signed::Signed;
