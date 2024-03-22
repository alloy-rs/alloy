/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug, thiserror::Error)]
#[allow(missing_copy_implementations)]
#[allow(missing_docs)]
pub enum ConversionError {
    #[error("missing `gasPrice` field for Legacy transaction")]
    MissingGasPrice,
    #[error("missing signature for transaction")]
    MissingSignature,
    #[error("missing `accessList` field for EIP-2930 transaction")]
    MissingAccessList,
    #[error("missing `maxFeePerGas` field for EIP-1559 transaction")]
    MissingMaxFeePerGas,
    #[error("missing `maxPriorityFeePerGas` field for EIP-1559 transaction")]
    MissingMaxPriorityFeePerGas,
    #[error("missing `maxFeePerBlobGas` field for EIP-1559 transaction")]
    MissingMaxFeePerBlobGas,
    #[error("missing `chainId` field for EIP-155 transaction")]
    MissingChainId,
    #[error("missing `to` field for EIP-4844 transaction")]
    MissingTo,
    #[error(transparent)]
    SignatureError(#[from] alloy_primitives::SignatureError),
    #[error(transparent)]
    Eip2718Error(#[from] alloy_eips::eip2718::Eip2718Error),
}
