use crate::error::Error as ContractError;
use alloy_transport::TransportError;

#[derive(Debug, thiserror::Error)]
pub enum MulticallError {
    #[error("Chain ID {0} is not supported by Multicall. Please use an address instead")]
    InvalidChainId(u64),

    #[error(transparent)]
    TransportError(#[from] TransportError),

    #[error(transparent)]
    ContractError(#[from] ContractError),

    #[error("Multicall call reverted but `allowFailure` is false")]
    FailedCall,
}
