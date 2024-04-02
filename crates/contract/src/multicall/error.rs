use crate::error::Error as ContractError;
use alloy_transport::TransportError;

/// Errors when interacting with a Multicall contract.
#[derive(Debug, thiserror::Error)]
pub enum MulticallError {
    /// Unsupported Chain ID used when creating the Multicall instance
    #[error("Chain ID {0} is not supported by Multicall. Please use an address instead")]
    InvalidChainId(u64),

    /// An error occurred interacting with a contract over RPC.
    #[error(transparent)]
    TransportError(#[from] TransportError),

    /// Error when interacting with contracts. This is an error from the `contract` crate.
    #[error(transparent)]
    ContractError(#[from] ContractError),

    /// Multicall reverted due to an individual call failing.
    #[error("Multicall call reverted but `allowFailure` is false")]
    FailedCall,

    /// Attempted to initialize the Multicall instance with no address or chain ID
    #[error("Invalid params. Must provide at least one of: address or chain_id.")]
    InvalidInitializationParams,
}
