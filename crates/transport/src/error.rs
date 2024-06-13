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

    /// HTTP Error with code and body
    #[error("{0}")]
    HttpError(#[from] HttpError),

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

    /// Instantiate a new `TrasnportError::HttpError`.
    pub const fn http_error(status: u16, body: String) -> TransportError {
        RpcError::Transport(Self::HttpError(HttpError { status, body }))
    }

    /// Analyzes the [TransportErrorKind] and decides if the request should be retried based on the
    /// variant.
    pub fn is_retry_err(&self) -> bool {
        match self {
            // Missing batch response errors can be retried.
            Self::MissingBatchResponse(_) => true,
            Self::HttpError(http_err) => http_err.is_rate_limit_err(),
            Self::Custom(err) => {
                let msg = err.to_string();
                msg.contains("429 Too Many Requests")
            }
            _ => false,
        }
    }
}

/// Type for holding HTTP errors such as 429 rate limit error.
#[derive(Debug, thiserror::Error)]
#[error("HTTP error {status} with body: {body}")]
pub struct HttpError {
    pub status: u16,
    pub body: String,
}

impl HttpError {
    /// Checks the `status` to determine whether the request should be retried.
    pub const fn is_rate_limit_err(&self) -> bool {
        if self.status == 429 {
            return true;
        }
        false
    }
}
