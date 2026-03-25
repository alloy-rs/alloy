#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod traits;
pub use traits::{
    BlockResponse, HeaderResponse, ReceiptResponse, TransactionFailedError, TransactionResponse,
};

mod block;
pub use block::{BlockTransactionHashes, BlockTransactions, BlockTransactionsKind};

mod tx_builders;
pub use tx_builders::{TransactionBuilder4844, TransactionBuilder7594, TransactionBuilder7702};

mod tx_meta;
pub use tx_meta::InclusionInfo;
