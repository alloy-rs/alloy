use alloy_json_rpc::{Id, RpcError, RpcResult};
use serde_json::value::RawValue;
use std::{error::Error as StdError, fmt::Debug};
use thiserror::Error;

/// A transport error is an [`RpcError`] containing a [`TransportErrorKind`].
pub type TransportError<ErrResp = Box<RawValue>> = RpcError<TransportErrorKind, ErrResp>;

/// A transport result is a [`Result`] containing a [`TransportError`].
pub type TransportResult<T, ErrResp = Box<RawValue>> = RpcResult<T, TransportErrorKind, ErrResp>;

/// Transport error.
///
/// All transport errors are wrapped in this enum.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransportErrorKind {
    /// Missing batch response.
    ///
    /// This error is returned when a batch request is sent and the response
    /// does not contain a response for a request. For convenience the ID is
    /// specified.
    #[error("missing response for request with ID {0}")]
    MissingBatchResponse(Id),

    /// Backend connection task has stopped.
    #[error("backend connection task has stopped")]
    BackendGone,

    /// Pubsub service is not available for the current provider.
    #[error("subscriptions are not available on this provider")]
    PubsubUnavailable,

    /// Custom error.
    #[error("{0}")]
    Custom(#[source] Box<dyn StdError + Send + Sync + 'static>),
}

impl TransportErrorKind {
    /// Returns `true` if the error is potentially recoverable.
    /// This is a naive heuristic and should be used with caution.
    pub const fn recoverable(&self) -> bool {
        matches!(self, Self::MissingBatchResponse(_))
    }

    /// Instantiate a new `TransportError` from a custom error.
    pub fn custom_str(err: &str) -> TransportError {
        RpcError::Transport(Self::Custom(err.into()))
    }

    /// Instantiate a new `TransportError` from a custom error.
    pub fn custom(err: impl StdError + Send + Sync + 'static) -> TransportError {
        RpcError::Transport(Self::Custom(Box::new(err)))
    }

    /// Instantiate a new `TransportError` from a missing ID.
    pub const fn missing_batch_response(id: Id) -> TransportError {
        RpcError::Transport(Self::MissingBatchResponse(id))
    }

    /// Instantiate a new `TransportError::BackendGone`.
    pub const fn backend_gone() -> TransportError {
        RpcError::Transport(Self::BackendGone)
    }

    /// Instantiate a new `TransportError::PubsubUnavailable`.
    pub const fn pubsub_unavailable() -> TransportError {
        RpcError::Transport(Self::PubsubUnavailable)
    }
}
