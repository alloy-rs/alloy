use core::num::TryFromIntError;

use alloc::string::String;

/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug, derive_more::Display)]
pub enum ConversionError {
    /// Error during EIP-2718 transaction coding.
    #[display("{_0}")]
    Eip2718Error(alloy_eips::eip2718::Eip2718Error),
    /// [`alloy_primitives::SignatureError`].
    #[display("{_0}")]
    SignatureError(alloy_primitives::SignatureError),
    /// Missing signature for transaction.
    #[display("missing signature for transaction")]
    MissingSignature,
    /// Missing y parity in signature.
    #[display("missing y parity in signature")]
    MissingYParity,
    /// Invalid signature
    #[display("invalid signature")]
    InvalidSignature,
    /// Missing `chainId` field for EIP-1559 transaction.
    #[display("missing `chainId` field for EIP-155 transaction")]
    MissingChainId,
    /// Missing `gasPrice` field for Legacy transaction.
    #[display("missing `gasPrice` field for Legacy transaction")]
    MissingGasPrice,
    /// Missing `accessList` field for EIP-2930 transaction.
    #[display("missing `accessList` field for EIP-2930 transaction")]
    MissingAccessList,
    /// Missing `maxFeePerGas` field for EIP-1559 transaction.
    #[display("missing `maxFeePerGas` field for EIP-1559 transaction")]
    MissingMaxFeePerGas,
    /// Missing `maxPriorityFeePerGas` field for EIP-1559 transaction.
    #[display("missing `maxPriorityFeePerGas` field for EIP-1559 transaction")]
    MissingMaxPriorityFeePerGas,
    /// Missing `maxFeePerBlobGas` field for EIP-1559 transaction.
    #[display("missing `maxFeePerBlobGas` field for EIP-1559 transaction")]
    MissingMaxFeePerBlobGas,
    /// Missing `to` field for EIP-4844 transaction.
    #[display("missing `to` field for EIP-4844 transaction")]
    MissingTo,
    /// Missing `blobVersionedHashes` field for EIP-4844 transaction.
    #[display("missing `blobVersionedHashes` field for EIP-4844 transaction")]
    MissingBlobVersionedHashes,
    /// Missing `authorizationList` field for EIP-7702 transaction.
    #[display("missing `authorizationList` field for EIP-7702 transaction")]
    MissingAuthorizationList,
    /// Missing full transactions required for block decoding
    #[display("missing full transactions required for block decoding")]
    MissingFullTransactions,
    /// Base fee per gas integer conversion error
    #[display("base fee per gas integer conversion error: {_0}")]
    BaseFeePerGasConversion(TryFromIntError),
    /// Gas limit integer conversion error
    #[display("gas limit integer conversion error: {_0}")]
    GasLimitConversion(TryFromIntError),
    /// Gas used integer conversion error
    #[display("gas used integer conversion error: {_0}")]
    GasUsedConversion(TryFromIntError),
    /// Missing block number
    #[display("missing block number")]
    MissingBlockNumber,
    /// Blob gas used integer conversion error
    #[display("blob gas used integer conversion error: {_0}")]
    BlobGasUsedConversion(TryFromIntError),
    /// Excess blob gas integer conversion error
    #[display("excess blob gas integer conversion error: {_0}")]
    ExcessBlobGasConversion(TryFromIntError),
    /// A custom Conversion Error that doesn't fit other categories.
    #[display("conversion error: {_0}")]
    Custom(String),
}

#[cfg(feature = "std")]
impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Eip2718Error(err) => Some(err),
            Self::SignatureError(err) => Some(err),
            Self::BaseFeePerGasConversion(err)
            | Self::GasLimitConversion(err)
            | Self::GasUsedConversion(err)
            | Self::BlobGasUsedConversion(err)
            | Self::ExcessBlobGasConversion(err) => Some(err),
            _ => None,
        }
    }
}

impl From<alloy_eips::eip2718::Eip2718Error> for ConversionError {
    fn from(err: alloy_eips::eip2718::Eip2718Error) -> Self {
        Self::Eip2718Error(err)
    }
}

impl From<alloy_primitives::SignatureError> for ConversionError {
    fn from(err: alloy_primitives::SignatureError) -> Self {
        Self::SignatureError(err)
    }
}
