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
            Self::Custom(err) => {
                // currently http error responses are not standard in alloy
                let msg = err.to_string();
                msg.contains("429 Too Many Requests")
            }
            Self::HttpError(http_err) => http_err.is_rate_limit_err(),

            // If the backend is gone, or there's a completely custom error, we should assume it's
            // not retryable.
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
    /// Analyzes the `status` and `body` to determine whether the request should be retried.
    pub fn is_rate_limit_err(&self) -> bool {
        // alchemy throws it this way
        if self.status == 429 {
            return true;
        }

        // alternative alchemy error for specific IPs
        if self.body.contains("rate limit") {
            return true;
        }

        // quick node error `"credits limited to 6000/sec"`
        // <https://github.com/foundry-rs/foundry/pull/6712#issuecomment-1951441240>
        if self.body.contains("credits") {
            return true;
        }

        // quick node rate limit error: `100/second request limit reached - reduce calls per second
        // or upgrade your account at quicknode.com` <https://github.com/foundry-rs/foundry/issues/4894>
        if self.body.contains("request limit reached") {
            return true;
        }

        match self.body.as_str() {
            // this is commonly thrown by infura and is apparently a load balancer issue, see also <https://github.com/MetaMask/metamask-extension/issues/7234>
            "header not found" => true,
            // also thrown by infura if out of budget for the day and ratelimited
            "daily request count exceeded, request rate limited" => true,
            msg => {
                msg.contains("rate limit")
                    || msg.contains("rate exceeded")
                    || msg.contains("too many requests")
                    || msg.contains("credits limited")
                    || msg.contains("request limit")
            }
        }
    }
}
