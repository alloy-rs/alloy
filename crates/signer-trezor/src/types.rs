//! Helpers for interacting with the Ethereum Trezor App.
//!
//! [Official Docs](https://docs.trezor.io/trezor-firmware/index.html)

use alloy_primitives::hex;
use std::fmt;
use thiserror::Error;

/// A Trezor derivation-path preset.
#[derive(Clone, Debug)]
pub enum DerivationType {
    /// Standard path `m/44'/60'/0'/0/<index>`.
    TrezorLive(usize),
    /// Any other path.
    ///
    /// **Warning**: Trezor by default forbids custom derivation paths;
    /// run `trezorctl set safety-checks prompt` to enable them.
    Other(String),
}

impl fmt::Display for DerivationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::TrezorLive(index) => write!(f, "m/44'/60'/0'/0/{index}"),
            Self::Other(inner) => f.write_str(inner),
        }
    }
}

#[derive(Debug, Error)]
/// Error when using the Trezor transport
pub enum TrezorError {
    /// Underlying Trezor transport error.
    #[error(transparent)]
    Client(#[from] trezor_client::error::Error),
    /// Thrown when converting from a hex string.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Thrown when converting a semver requirement.
    #[error(transparent)]
    Semver(#[from] semver::Error),
    /// Signature Error
    #[error(transparent)]
    SignatureError(#[from] alloy_primitives::SignatureError),
    /// Device firmware is older than the minimum accepted during signer construction.
    #[error("Trezor Ethereum app requires at least version {0:?}")]
    UnsupportedFirmwareVersion(String),
    /// Need to provide a chain ID for EIP-155 signing.
    #[error("missing Trezor signer chain ID")]
    MissingChainId,
    /// Unsupported transaction type for Trezor signing.
    #[error(
        "Trezor transaction signing only supports legacy and EIP-1559 transactions; got transaction type 0x{0:02x}"
    )]
    UnsupportedTransactionType(u8),
    /// Could not retrieve device features.
    #[error("could not retrieve device features")]
    Features,
    /// Invalid derivation path.
    #[error("invalid derivation path: {0}")]
    InvalidDerivationPath(String),
}

impl From<TrezorError> for alloy_signer::Error {
    fn from(error: TrezorError) -> Self {
        Self::other(error)
    }
}
