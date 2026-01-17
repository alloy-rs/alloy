use alloy_dyn_abi::Error as AbiError;
use alloy_primitives::{Bytes, Selector};
use alloy_provider::PendingTransactionError;
use alloy_sol_types::{SolError, SolInterface};
use alloy_transport::{RpcError, TransportError, TransportErrorKind};
use serde_json::value::RawValue;
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
    /// The contract returned no data.
    #[error("contract call to `{0}` returned no data (\"0x\"); the called address might not be a contract")]
    ZeroData(String, #[source] AbiError),
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

impl Error {
    #[cold]
    pub(crate) fn decode(name: &str, data: &[u8], error: AbiError) -> Self {
        if data.is_empty() {
            let name = name.split('(').next().unwrap_or(name);
            return Self::ZeroData(name.to_string(), error);
        }
        Self::AbiError(error)
    }

    /// Return the revert data in case the call reverted.
    pub fn as_revert_data(&self) -> Option<Bytes> {
        if let Self::TransportError(e) = self {
            return e.as_error_resp().and_then(|e| e.as_revert_data());
        }

        None
    }

    /// Attempts to decode the revert data into one of the custom errors in [`SolInterface`].
    ///
    /// Returns an enum container type consisting of the custom errors defined in the interface.
    ///
    /// None is returned if the revert data is empty or if the data could not be decoded into one of
    /// the custom errors defined in the interface.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alloy_provider::ProviderBuilder;
    /// use alloy_sol_types::sol;
    ///
    /// sol! {
    ///     #[derive(Debug, PartialEq, Eq)]
    ///     #[sol(rpc, bytecode = "694207")]
    ///     contract ThrowsError {
    ///         error SomeCustomError(uint64 a);
    ///         error AnotherError(uint64 b);
    ///
    ///         function error(uint64 a) external {
    ///             revert SomeCustomError(a);
    ///         }
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    ///
    ///     let throws_err = ThrowsError::deploy(provider).await.unwrap();
    ///
    ///     let err = throws_err.error(42).call().await.unwrap_err();
    ///
    ///     let custom_err =
    ///         err.as_decoded_interface_error::<ThrowsError::ThrowsErrorErrors>().unwrap();
    ///
    ///     // Handle the custom error enum
    ///     match custom_err {
    ///         ThrowsError::ThrowsErrorErrors::SomeCustomError(a) => { /* handle error */ }
    ///         ThrowsError::ThrowsErrorErrors::AnotherError(b) => { /* handle error */ }
    ///     }
    /// }
    /// ```
    pub fn as_decoded_interface_error<E: SolInterface>(&self) -> Option<E> {
        self.as_revert_data().and_then(|data| E::abi_decode(&data).ok())
    }

    /// Try to decode a contract error into a specific Solidity error interface.
    /// If the error cannot be decoded or it is not a contract error, return the original error.
    ///
    /// Example usage:
    ///
    /// ```ignore
    /// sol! {
    ///    library ErrorLib {
    ///       error SomeError(uint256 code);
    ///    }
    /// }
    ///
    /// // call a contract that may return an error with the SomeError interface
    /// let returndata = match myContract.call().await {
    ///    Ok(returndata) => returndata,
    ///    Err(err) => {
    ///         let decoded_error = err.try_decode_into_interface_error::<ErrorLib::ErrorLibError>()?;
    ///        // handle the decoded error however you want; for example, return it
    ///         return Err(decoded_error);
    ///    },
    /// }
    /// ```
    ///
    /// See also [`Self::as_decoded_interface_error`] for more details.
    pub fn try_decode_into_interface_error<I: SolInterface>(self) -> Result<I, Self> {
        self.as_decoded_interface_error::<I>().ok_or(self)
    }

    /// Decode the revert data into a custom [`SolError`] type.
    ///
    /// Returns an instance of the custom error type if decoding was successful, otherwise None.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alloy_provider::ProviderBuilder;
    /// use alloy_sol_types::sol;
    /// use ThrowsError::SomeCustomError;
    /// sol! {
    ///     #[derive(Debug, PartialEq, Eq)]
    ///     #[sol(rpc, bytecode = "694207")]
    ///     contract ThrowsError {
    ///         error SomeCustomError(uint64 a);
    ///         error AnotherError(uint64 b);
    ///
    ///         function error(uint64 a) external {
    ///             revert SomeCustomError(a);
    ///         }
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    ///
    ///     let throws_err = ThrowsError::deploy(provider).await.unwrap();
    ///
    ///     let err = throws_err.error(42).call().await.unwrap_err();
    ///
    ///     let custom_err = err.as_decoded_error::<SomeCustomError>().unwrap();
    ///
    ///     assert_eq!(custom_err, SomeCustomError { a: 42 });
    /// }
    /// ```
    pub fn as_decoded_error<E: SolError>(&self) -> Option<E> {
        self.as_revert_data().and_then(|data| E::abi_decode(&data).ok())
    }
}

/// The result of trying to parse a transport error into a specific interface.
#[derive(Debug)]
pub enum TryParseTransportErrorResult<I: SolInterface> {
    /// The error was successfully decoded into the specified interface.
    Decoded(I),
    /// The error was not decoded but the revert data was extracted.
    UnknownSelector(Bytes),
    /// The error was not decoded and the revert data was not extracted.
    Original(RpcError<TransportErrorKind, Box<RawValue>>),
}

/// Extension trait for TransportError parsing capabilities
pub trait TransportErrorExt {
    /// Attempts to parse a transport error into a specific interface.
    fn try_parse_transport_error<I: SolInterface>(self) -> TryParseTransportErrorResult<I>;
}

impl TransportErrorExt for TransportError {
    fn try_parse_transport_error<I: SolInterface>(self) -> TryParseTransportErrorResult<I> {
        let revert_data = self.as_error_resp().and_then(|e| e.as_revert_data());
        if let Some(decoded) =
            revert_data.as_ref().and_then(|data| I::abi_decode(data.as_ref()).ok())
        {
            return TryParseTransportErrorResult::Decoded(decoded);
        }

        if let Some(decoded) = revert_data {
            return TryParseTransportErrorResult::UnknownSelector(decoded);
        }
        TryParseTransportErrorResult::Original(self)
    }
}
