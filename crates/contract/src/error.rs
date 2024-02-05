use alloy_dyn_abi::Error as AbiError;
use alloy_primitives::Selector;
use alloy_transport::TransportError;
use std::fmt;

/// Dynamic contract result type.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Error when interacting with contracts.
#[derive(Debug)]
pub enum Error {
    /// Unknown function referenced.
    UnknownFunction(String),
    /// Unknown function selector referenced.
    UnknownSelector(Selector),
    /// An error occurred ABI encoding or decoding.
    AbiError(AbiError),
    /// An error occurred interacting with a contract over RPC.
    TransportError(TransportError),
}

impl From<AbiError> for Error {
    #[inline]
    fn from(error: AbiError) -> Self {
        Self::AbiError(error)
    }
}

impl From<TransportError> for Error {
    #[inline]
    fn from(error: TransportError) -> Self {
        Self::TransportError(error)
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AbiError(e) => Some(e),
            Self::TransportError(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFunction(name) => {
                write!(f, "unknown function: function {name} does not exist",)
            }
            Self::UnknownSelector(selector) => {
                write!(f, "unknown function: function with selector {selector} does not exist")
            }

            Self::AbiError(e) => e.fmt(f),
            Self::TransportError(e) => e.fmt(f),
        }
    }
}
