use core::num::TryFromIntError;

use alloc::string::String;

/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum ConversionError {
    /// Error during EIP-2718 transaction coding.
    #[cfg_attr(feature = "std", error(transparent))]
    Eip2718Error(#[cfg_attr(feature = "std", from)] alloy_eips::eip2718::Eip2718Error),
    /// [`alloy_primitives::SignatureError`].
    #[cfg_attr(feature = "std", error(transparent))]
    SignatureError(#[cfg_attr(feature = "std", from)] alloy_primitives::SignatureError),
    /// Missing signature for transaction.
    #[cfg_attr(feature = "std", error("missing signature for transaction"))]
    MissingSignature,
    /// Missing y parity in signature.
    #[cfg_attr(feature = "std", error("missing y parity in signature"))]
    MissingYParity,
    /// Invalid signature
    #[cfg_attr(feature = "std", error("invalid signature"))]
    InvalidSignature,
    /// Missing `chainId` field for EIP-1559 transaction.
    #[cfg_attr(feature = "std", error("missing `chainId` field for EIP-155 transaction"))]
    MissingChainId,
    /// Missing `gasPrice` field for Legacy transaction.
    #[cfg_attr(feature = "std", error("missing `gasPrice` field for Legacy transaction"))]
    MissingGasPrice,
    /// Missing `accessList` field for EIP-2930 transaction.
    #[cfg_attr(feature = "std", error("missing `accessList` field for EIP-2930 transaction"))]
    MissingAccessList,
    /// Missing `maxFeePerGas` field for EIP-1559 transaction.
    #[cfg_attr(feature = "std", error("missing `maxFeePerGas` field for EIP-1559 transaction"))]
    MissingMaxFeePerGas,
    /// Missing `maxPriorityFeePerGas` field for EIP-1559 transaction.
    #[cfg_attr(
        feature = "std",
        error("missing `maxPriorityFeePerGas` field for EIP-1559 transaction")
    )]
    MissingMaxPriorityFeePerGas,
    /// Missing `maxFeePerBlobGas` field for EIP-1559 transaction.
    #[cfg_attr(
        feature = "std",
        error("missing `maxFeePerBlobGas` field for EIP-1559 transaction")
    )]
    MissingMaxFeePerBlobGas,
    /// Missing `to` field for EIP-4844 transaction.
    #[cfg_attr(feature = "std", error("missing `to` field for EIP-4844 transaction"))]
    MissingTo,
    /// Missing `blobVersionedHashes` field for EIP-4844 transaction.
    #[cfg_attr(
        feature = "std",
        error("missing `blobVersionedHashes` field for EIP-4844 transaction")
    )]
    MissingBlobVersionedHashes,
    /// Missing `authorizationList` field for EIP-7702 transaction.
    #[cfg_attr(
        feature = "std",
        error("missing `authorizationList` field for EIP-7702 transaction")
    )]
    MissingAuthorizationList,
    /// Missing full transactions required for block decoding
    #[cfg_attr(feature = "std", error("missing full transactions required for block decoding"))]
    MissingFullTransactions,
    /// Base fee per gas integer conversion error
    #[cfg_attr(feature = "std", error("base fee per gas integer conversion error: {0}"))]
    BaseFeePerGasConversion(TryFromIntError),
    /// Gas limit integer conversion error
    #[cfg_attr(feature = "std", error("gas limit integer conversion error: {0}"))]
    GasLimitConversion(TryFromIntError),
    /// Gas used integer conversion error
    #[cfg_attr(feature = "std", error("gas used integer conversion error: {0}"))]
    GasUsedConversion(TryFromIntError),
    /// Missing block number
    #[cfg_attr(feature = "std", error("missing block number"))]
    MissingBlockNumber,
    /// Blob gas used integer conversion error
    #[cfg_attr(feature = "std", error("blob gas used integer conversion error: {0}"))]
    BlobGasUsedConversion(TryFromIntError),
    /// Excess blob gas integer conversion error
    #[cfg_attr(feature = "std", error("excess blob gas integer conversion error: {0}"))]
    ExcessBlobGasConversion(TryFromIntError),
    /// A custom Conversion Error that doesn't fit other categories.
    #[cfg_attr(feature = "std", error("conversion error: {0}"))]
    Custom(String),
}

#[cfg(not(feature = "std"))]
impl From<alloy_primitives::SignatureError> for ConversionError {
    fn from(e: alloy_primitives::SignatureError) -> Self {
        ConversionError::SignatureError(e)
    }
}

#[cfg(not(feature = "std"))]
impl From<alloy_eips::eip2718::Eip2718Error> for ConversionError {
    fn from(e: alloy_eips::eip2718::Eip2718Error) -> Self {
        ConversionError::Eip2718Error(e)
    }
}
