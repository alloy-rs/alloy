/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[allow(missing_copy_implementations)]
#[allow(missing_docs)]
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
}
