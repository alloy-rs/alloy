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

mod signature;
pub use signature::Signature;

mod signer;
pub use signer::Signer;

mod wallet;
pub use wallet::{MnemonicBuilder, Wallet, WalletError};

pub mod utils;

#[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
pub use yubihsm;

/// Re-export the BIP-32 crate so that wordlists can be accessed conveniently.
pub use coins_bip39;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = Wallet<k256::ecdsa::SigningKey>;

/// A wallet instantiated with a YubiHSM
#[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
pub type YubiWallet = Wallet<yubihsm::ecdsa::Signer<k256::Secp256k1>>;
