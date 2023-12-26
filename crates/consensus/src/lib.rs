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

mod basefee;
pub use basefee::BaseFeeParams;

pub mod constants;

mod header;
pub use header::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod pure;
pub use pure::{calc_blob_gasprice, calc_excess_blob_gas, calc_next_block_base_fee};

mod receipt;
pub use receipt::{Receipt, ReceiptEnvelope, ReceiptWithBloom};

mod transaction;
pub use transaction::{
    AccessList, AccessListItem, TxEip1559, TxEip2930, TxEnvelope, TxKind, TxLegacy, TxType,
};
