#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod block;
pub use block::AnyHeader;

mod receipt;
pub use receipt::AnyReceiptEnvelope;

// Unknown transaction types require std (OnceLock) and serde (OtherFields).
// They are gated behind the std feature which enables those deps.
#[cfg(feature = "std")]
mod unknown;
#[cfg(feature = "std")]
pub use unknown::{AnyTxType, DeserMemo, UnknownTxEnvelope, UnknownTypedTransaction};
