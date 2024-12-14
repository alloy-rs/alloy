use alloy_dyn_abi::Error as AbiError;
use alloy_primitives::Selector;
use alloy_provider::PendingTransactionError;
use alloy_transport::TransportError;
use thiserror::Error;

/// Dynamic contract result type.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Error when interacting with contracts.
#[derive(Debug, Error)]
pub enum Error {
    /// Unknown function referenced.
    #[error("unknown function: function {0} does not exist")]
    UnknownFunction(String),
    /// Unknown function selector referenced.
    #[error("unknown function: function with selector {0} does not exist")]
    UnknownSelector(Selector),
    /// Called `deploy` with a transaction that is not a deployment transaction.
    #[error("transaction is not a deployment transaction")]
    NotADeploymentTransaction,
    /// `contractAddress` was not found in the deployment transaction’s receipt.
    #[error("missing `contractAddress` from deployment transaction receipt")]
    ContractNotDeployed,
    /// An error occurred ABI encoding or decoding.
    #[error(transparent)]
    AbiError(#[from] AbiError),
    /// An error occurred interacting with a contract over RPC.
    #[error(transparent)]
    TransportError(#[from] TransportError),
    /// An error occurred while waiting for a pending transaction.
    #[error(transparent)]
    PendingTransactionError(#[from] PendingTransactionError),
}

impl From<alloy_sol_types::Error> for Error {
    #[inline]
    fn from(e: alloy_sol_types::Error) -> Self {
        Self::AbiError(e.into())
    }
}
