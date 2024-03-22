#[allow(missing_docs)]
/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// Missing `gasPrice` field for Legacy transaction.
    #[error("missing `gasPrice` field for Legacy transaction.")]
    MissingGasPrice,
    /// Missing signature for transaction.
    #[error("missing signature for transaction.")]
    MissingSignature,
    /// Missing `accessList` field for EIP-2930 transaction.
    #[error("missing `accessList` field for EIP-2930 transaction.")]
    MissingAccessList,
    /// Missing `maxFeePerGas` field for EIP-1559 transaction.
    #[error("missing `maxFeePerGas` field for EIP-1559 transaction.")]
    MissingMaxFeePerGas,
    /// Missing `to` field for EIP-4844 transaction.
    #[error("missing `to` field for EIP-4844 transaction.")]
    MissingTo,
    /// Missing `maxPriorityFeePerGas` field for EIP-1559 transaction.
    #[error("missing `maxPriorityFeePerGas` field for EIP-1559 transaction.")]
    MissingMaxPriorityFeePerGas,
    /// Missing `maxFeePerBlobGas` field for EIP-1559 transaction.
    #[error("missing `maxFeePerBlobGas` field for EIP-1559 transaction.")]
    MissingMaxFeePerBlobGas,
    /// Missing `chainId` field for EIP-1559 transaction.
    #[error("missing `chainId` field for EIP-155 transaction.")]
    MissingChainId,
    /// Error during signature parsing.
    #[error(transparent)]
    SignatureError(#[from] alloy_primitives::SignatureError),
    /// Error during EIP-2718 transaction coding.
    #[error(transparent)]
    Eip2718Error(#[from] alloy_eips::eip2718::Eip2718Error),
}
