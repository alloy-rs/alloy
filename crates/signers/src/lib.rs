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
// #![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

// #[macro_use]
// extern crate tracing;

mod signature;
pub use signature::Signature;

mod signer;
pub use signer::Signer;

mod wallet;
pub use wallet::{MnemonicBuilder, Wallet, WalletError};

// #[cfg(all(feature = "ledger", not(target_arch = "wasm32")))]
// mod ledger;
// #[cfg(all(feature = "ledger", not(target_arch = "wasm32")))]
// pub use ledger::{
//     app::LedgerEthereum as Ledger,
//     types::{DerivationType as HDPath, LedgerError},
// };

// #[cfg(all(feature = "trezor", not(target_arch = "wasm32")))]
// mod trezor;
// #[cfg(all(feature = "trezor", not(target_arch = "wasm32")))]
// pub use trezor::{
//     app::TrezorEthereum as Trezor,
//     types::{DerivationType as TrezorHDPath, TrezorError},
// };

// #[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
// pub use yubihsm;

// #[cfg(feature = "aws")]
// mod aws;
// #[cfg(feature = "aws")]
// pub use aws::{AwsSigner, AwsSignerError};

pub mod utils;

/// Re-export the BIP-32 crate so that wordlists can be accessed conveniently.
pub use coins_bip39;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = Wallet<k256::ecdsa::SigningKey>;

#[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
/// A wallet instantiated with a YubiHSM
pub type YubiWallet = Wallet<yubihsm::ecdsa::Signer<k256::Secp256k1>>;
