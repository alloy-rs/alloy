/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug, thiserror::Error)]
#[allow(missing_copy_implementations)]
#[allow(missing_docs)]
pub enum ConversionError {
    /// Error during EIP-2718 transaction coding.
    #[error(transparent)]
    Eip2718Error(#[from] alloy_eips::eip2718::Eip2718Error),
    /// [`alloy_primitives::SignatureError`].
    #[error(transparent)]
    SignatureError(#[from] alloy_primitives::SignatureError),
    /// Missing signature for transaction.
    #[error("missing signature for transaction")]
    MissingSignature,
    /// Missing `chainId` field for EIP-1559 transaction.
    #[error("missing `chainId` field for EIP-155 transaction")]
    MissingChainId,
    /// Missing `gasPrice` field for Legacy transaction.
    #[error("missing `gasPrice` field for Legacy transaction")]
    MissingGasPrice,
    /// Missing `accessList` field for EIP-2930 transaction.
    #[error("missing `accessList` field for EIP-2930 transaction")]
    MissingAccessList,
    /// Missing `maxFeePerGas` field for EIP-1559 transaction.
    #[error("missing `maxFeePerGas` field for EIP-1559 transaction")]
    MissingMaxFeePerGas,
    /// Missing `maxPriorityFeePerGas` field for EIP-1559 transaction.
    #[error("missing `maxPriorityFeePerGas` field for EIP-1559 transaction")]
    MissingMaxPriorityFeePerGas,
    /// Missing `maxFeePerBlobGas` field for EIP-1559 transaction.
    #[error("missing `maxFeePerBlobGas` field for EIP-1559 transaction")]
    MissingMaxFeePerBlobGas,
    /// Missing `to` field for EIP-4844 transaction.
    #[error("missing `to` field for EIP-4844 transaction")]
    MissingTo,
    /// Missing `blobVersionedHashes` field for EIP-4844 transaction.
    #[error("missing `blobVersionedHashes` field for EIP-4844 transaction")]
    MissingBlobVersionedHashes,
    /// Missing full transactions required for block decoding
    #[error("missing full transactions required for block decoding")]
    MissingFullTransactions,
    /// Base fee per gas conversion error
    #[error("base fee per gas conversion error")]
    BaseFeePerGasConversion,
    /// Gas limit conversion error
    #[error("gas limit conversion error")]
    GasLimitConversion,
    /// Gas used conversion error
    #[error("gas used conversion error")]
    GasUsedConversion,
    /// Missing block number
    #[error("missing block number")]
    MissingBlockNumber,
    /// Block number conversion error
    #[error("block number conversion error")]
    BlockNumberConversion,
    /// Timestamp conversion error
    #[error("timestamp conversion error")]
    TimestampConversion,
}
