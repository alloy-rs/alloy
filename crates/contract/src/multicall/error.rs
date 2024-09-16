#[derive(Debug, thiserror::Error)]
/// Errors that can occur when interacting with the Multicall contract
#[allow(missing_docs)]
pub enum MultiCallError {
    #[error("A call with no target address was attempted to be added to the multicall")]
    MissingTargetAddress,

    #[error("The multicall contract is not deployed on the current chain")]
    ChainNotSupported,

    #[error(transparent)]
    ContractError(#[from] crate::Error),
}
