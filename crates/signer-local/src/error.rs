use alloy_primitives::hex;
use thiserror::Error;

/// Error thrown by [`LocalSigner`](crate::LocalSigner).
#[derive(Debug, Error)]
pub enum LocalSignerError {
    /// [`ecdsa`] error.
    #[cfg(feature = "k256")]
    #[error(transparent)]
    EcdsaError(#[from] k256::ecdsa::Error),
    /// [`hex`](mod@hex) error.
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    /// [`std::io`] error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    /// [`coins_bip32`] error.
    #[error(transparent)]
    #[cfg(feature = "mnemonic")]
    Bip32Error(#[from] coins_bip32::Bip32Error),
    /// [`coins_bip39`] error.
    #[error(transparent)]
    #[cfg(feature = "mnemonic")]
    Bip39Error(#[from] coins_bip39::MnemonicError),
    /// [`MnemonicBuilder`](super::mnemonic::MnemonicBuilder) error.
    #[error(transparent)]
    #[cfg(feature = "mnemonic")]
    MnemonicBuilderError(#[from] super::mnemonic::MnemonicBuilderError),

    /// [`secp256k1`] error.
    #[cfg(feature = "secp256k1")]
    #[error(transparent)]
    Secp256k1Error(#[from] secp256k1::Error),

    /// [`eth_keystore`] error.
    #[cfg(feature = "keystore")]
    #[error(transparent)]
    EthKeystoreError(#[from] eth_keystore::KeystoreError),
}
