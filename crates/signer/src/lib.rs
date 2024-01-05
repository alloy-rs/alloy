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

mod error;
pub use error::{Error, Result, UnsupportedSignerOperation};

mod signer;
pub use signer::{SignableTx, Signer, SignerSync, TransactionExt};

mod wallet;
#[cfg(feature = "mnemonic")]
pub use wallet::MnemonicBuilder;
pub use wallet::{Wallet, WalletError};

pub mod utils;

pub use alloy_primitives::Signature;

#[cfg(feature = "yubihsm")]
pub use yubihsm;

#[cfg(feature = "mnemonic")]
pub use coins_bip39;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = Wallet<k256::ecdsa::SigningKey>;

/// A wallet instantiated with a YubiHSM
#[cfg(feature = "yubihsm")]
pub type YubiWallet = Wallet<yubihsm::ecdsa::Signer<k256::Secp256k1>>;
