use alloy_primitives::hex;
use k256::ecdsa;
use thiserror::Error;

/// Result type alias for [`SignerError`].
pub type SignerResult<T, E = SignerError> = std::result::Result<T, E>;

/// Generic error type for [`Signer`](crate::Signer) implementations.
#[derive(Debug, Error)]
pub enum SignerError {
    /// This operation is not supported by the signer.
    #[error("signer operation {0} not supported")]
    UnsupportedOperation, // TODO: enum UnsupportedOperation ?
    /// Mismatch between provided transaction chain ID and signer chain ID.
    #[error("")]
    TransactionChainIdMismatch(u64, u64),
    /// [`ecdsa`] error.
    #[error(transparent)]
    Ecdsa(#[from] ecdsa::Error),
    /// [`hex`] error.
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    /// Generic error.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl SignerError {
    pub fn new(error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self::Other(error.into())
    }
}

// impl<T> From<T> for SignerError
// where
//     Box<dyn std::error::Error + Send + Sync + 'static>: From<T>,
// {
//     fn from(value: T) -> Self {
//         Self::Other(Box::from(value))
//     }
// }
