use alloy_json_rpc::{ErrorPayload, Id, RpcError, RpcResult};
use serde::Deserialize;
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

    /// Instantiate a new `TransportError::HttpError`.
    pub const fn http_error(status: u16, body: String) -> TransportError {
        RpcError::Transport(Self::HttpError(HttpError { status, body }))
    }

    /// Returns true if this is [`TransportErrorKind::PubsubUnavailable`].
    pub const fn is_pubsub_unavailable(&self) -> bool {
        matches!(self, Self::PubsubUnavailable)
    }

    /// Returns true if this is [`TransportErrorKind::BackendGone`].
    pub const fn is_backend_gone(&self) -> bool {
        matches!(self, Self::BackendGone)
    }

    /// Returns true if this is [`TransportErrorKind::HttpError`].
    pub const fn is_http_error(&self) -> bool {
        matches!(self, Self::HttpError(_))
    }

    /// Returns the [`HttpError`] if this is [`TransportErrorKind::HttpError`].
    pub const fn as_http_error(&self) -> Option<&HttpError> {
        match self {
            Self::HttpError(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the custom error if this is [`TransportErrorKind::Custom`].
    pub const fn as_custom(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        match self {
            Self::Custom(err) => Some(&**err),
            _ => None,
        }
    }

    /// Analyzes the [TransportErrorKind] and decides if the request should be retried based on the
    /// variant.
    pub fn is_retry_err(&self) -> bool {
        match self {
            // Missing batch response errors can be retried.
            Self::MissingBatchResponse(_) => true,
            Self::HttpError(http_err) => {
                http_err.is_rate_limit_err() || http_err.is_temporarily_unavailable()
            }
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
#[error(
    "HTTP error {status} with {}",
    if body.is_empty() { "empty body".to_string() } else { format!("body: {body}") }
)]
pub struct HttpError {
    /// The HTTP status code.
    pub status: u16,
    /// The HTTP response body.
    pub body: String,
}

impl HttpError {
    /// Checks the `status` to determine whether the request should be retried.
    pub const fn is_rate_limit_err(&self) -> bool {
        self.status == 429
    }

    /// Checks the `status` to determine whether the service was temporarily unavailable and should
    /// be retried.
    pub const fn is_temporarily_unavailable(&self) -> bool {
        self.status == 503
    }
}

/// Extension trait for classifying [`TransportError`] values.
pub trait RpcErrorExt {
    /// Analyzes whether to retry the request depending on the error.
    fn is_retryable(&self) -> bool;

    /// Fetches the backoff hint from the error message if present.
    fn backoff_hint(&self) -> Option<std::time::Duration>;

    /// Returns `true` if the error indicates the transaction is already known.
    fn is_already_known(&self) -> bool;

    /// Returns `true` if the error indicates the replacement transaction is underpriced.
    fn is_replacement_underpriced(&self) -> bool;

    /// Returns `true` if the error indicates the transaction is underpriced.
    fn is_transaction_underpriced(&self) -> bool;

    /// Returns `true` if the error indicates the nonce is too low.
    fn is_nonce_too_low(&self) -> bool;
}

impl RpcErrorExt for TransportError {
    fn is_retryable(&self) -> bool {
        match self {
            // There was a transport-level error. This is either a non-retryable error,
            // or a server error that should be retried.
            Self::Transport(err) => err.is_retry_err(),
            // The transport could not serialize the error itself. The request was malformed from
            // the start.
            Self::SerError(_) => false,
            Self::DeserError { text, .. } => {
                if let Ok(resp) = serde_json::from_str::<ErrorPayload>(text) {
                    return resp.is_retry_err();
                }

                // some providers send invalid JSON RPC in the error case (no `id:u64`), but the
                // text should be a `JsonRpcError`
                #[derive(Deserialize)]
                struct Resp {
                    error: ErrorPayload,
                }

                if let Ok(resp) = serde_json::from_str::<Resp>(text) {
                    return resp.error.is_retry_err();
                }

                false
            }
            Self::ErrorResp(err) => err.is_retry_err(),
            Self::NullResp => true,
            _ => false,
        }
    }

    fn backoff_hint(&self) -> Option<std::time::Duration> {
        if let Self::ErrorResp(resp) = self {
            // try to extract backoff from the error data (infura-style)
            let data = resp.try_data_as::<serde_json::Value>();
            if let Some(Ok(data)) = data {
                // if daily rate limit exceeded, infura returns the requested backoff in the error
                // response
                let backoff_seconds = &data["rate"]["backoff_seconds"];
                // infura rate limit error
                if let Some(seconds) = backoff_seconds.as_u64() {
                    return Some(std::time::Duration::from_secs(seconds));
                }
                if let Some(seconds) = backoff_seconds.as_f64() {
                    return Some(std::time::Duration::from_secs(seconds as u64 + 1));
                }
            }

            // try to extract backoff from the error message, e.g. "try again in 4ms"
            if let Some(duration) = parse_retry_after(&resp.message) {
                return Some(duration);
            }
        }
        None
    }

    fn is_already_known(&self) -> bool {
        // see also: op-geth: https://github.com/ethereum-optimism/op-geth/blob/e666543dc5500428ee7c940e54263fe4968c5efd/core/txpool/legacypool/legacypool.go#L991-L993
        // reth: https://github.com/paradigmxyz/reth/blob/a3b749676c6c748bf977983c189f9f4c4f9e9fbe/crates/rpc/rpc-eth-types/src/error/mod.rs#L663-L665
        self.as_error_resp().map(|err| err.message == "already known").unwrap_or_default()
    }

    fn is_replacement_underpriced(&self) -> bool {
        // see also: geth: https://github.com/ethereum/go-ethereum/blob/a56558d0920b74b6553185de4aff79c3de534e01/core/txpool/errors.go#L38-L38
        self.as_error_resp()
            .map(|err| err.message.contains("replacement transaction underpriced"))
            .unwrap_or_default()
    }

    fn is_transaction_underpriced(&self) -> bool {
        // see also: geth: https://github.com/ethereum/go-ethereum/blob/a56558d0920b74b6553185de4aff79c3de534e01/core/txpool/errors.go#L34-L34
        self.as_error_resp()
            .map(|err| err.message.contains("transaction underpriced"))
            .unwrap_or_default()
    }

    fn is_nonce_too_low(&self) -> bool {
        // see also: geth: https://github.com/ethereum/go-ethereum/blob/85077be58edea572f29c3b1a6a055077f1a56a8b/core/error.go#L45-L47
        self.as_error_resp().map(|err| err.message.contains("nonce too low")).unwrap_or_default()
    }
}

/// Parses a duration from messages like "try again in 4ms", "try again in 1s".
fn parse_retry_after(message: &str) -> Option<std::time::Duration> {
    let after = message.split_once("try again in ")?.1.trim_start();

    let digits_len = after.as_bytes().iter().take_while(|b| b.is_ascii_digit()).count();
    let (digits, rest) = after.split_at(digits_len);
    let value: u64 = digits.parse().ok()?;

    let unit = rest.trim().trim_end_matches(|c: char| c.is_ascii_punctuation());
    match unit {
        "ms" => Some(std::time::Duration::from_millis(value)),
        "s" => Some(std::time::Duration::from_secs(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_retry_error() {
        let err = "{\"code\":-32007,\"message\":\"100/second request limit reached - reduce calls per second or upgrade your account at quicknode.com\"}";
        let err = serde_json::from_str::<ErrorPayload>(err).unwrap();
        assert!(TransportError::ErrorResp(err).is_retryable());
    }

    #[test]
    fn test_retry_error_rate_limited() {
        let err = r#"{"code":-32005,"message":"rate limited, try again in 4ms","data":null}"#;
        let err = serde_json::from_str::<ErrorPayload>(err).unwrap();
        let err = TransportError::ErrorResp(err);
        assert!(err.is_retryable());
        assert_eq!(err.backoff_hint(), Some(std::time::Duration::from_millis(4)));
    }

    #[test]
    fn parse_retry_after_millis() {
        assert_eq!(
            parse_retry_after("try again in 4ms"),
            Some(std::time::Duration::from_millis(4))
        );
        assert_eq!(
            parse_retry_after("rate limited, try again in 100ms"),
            Some(std::time::Duration::from_millis(100))
        );
    }

    #[test]
    fn parse_retry_after_seconds() {
        assert_eq!(parse_retry_after("try again in 2s"), Some(std::time::Duration::from_secs(2)));
    }

    #[test]
    fn parse_retry_after_none() {
        assert_eq!(parse_retry_after("some other error"), None);
        assert_eq!(parse_retry_after("try again in"), None);
        assert_eq!(parse_retry_after("try again in ms"), None);
        assert_eq!(parse_retry_after("try again in 4us"), None);
    }

    #[test]
    fn test_retry_error_429() {
        let err = r#"{"code":429,"event":-33200,"message":"Too Many Requests","details":"You have surpassed your allowed throughput limit. Reduce the amount of requests per second or upgrade for more capacity."}"#;
        let err = serde_json::from_str::<ErrorPayload>(err).unwrap();
        assert!(TransportError::ErrorResp(err).is_retryable());
    }

    #[test]
    fn detects_already_known() {
        let err = TransportError::ErrorResp(ErrorPayload {
            code: -32000,
            message: Cow::Borrowed("already known"),
            data: None,
        });

        assert!(err.is_already_known());
        assert!(!err.is_replacement_underpriced());
        assert!(!err.is_transaction_underpriced());
        assert!(!err.is_nonce_too_low());
    }

    #[test]
    fn detects_replacement_underpriced() {
        let err = TransportError::ErrorResp(ErrorPayload {
            code: -32000,
            message: Cow::Borrowed("replacement transaction underpriced"),
            data: None,
        });

        assert!(err.is_replacement_underpriced());
        assert!(err.is_transaction_underpriced());
        assert!(!err.is_already_known());
        assert!(!err.is_nonce_too_low());
    }

    #[test]
    fn detects_transaction_underpriced() {
        let err = TransportError::ErrorResp(ErrorPayload {
            code: -32000,
            message: Cow::Borrowed("transaction underpriced"),
            data: None,
        });

        assert!(err.is_transaction_underpriced());
        assert!(!err.is_replacement_underpriced());
        assert!(!err.is_already_known());
        assert!(!err.is_nonce_too_low());
    }

    #[test]
    fn detects_nonce_too_low() {
        let err = TransportError::ErrorResp(ErrorPayload {
            code: -32000,
            message: Cow::Borrowed("nonce too low"),
            data: None,
        });

        assert!(err.is_nonce_too_low());
        assert!(!err.is_already_known());
        assert!(!err.is_replacement_underpriced());
        assert!(!err.is_transaction_underpriced());
    }

    #[test]
    fn ignores_non_error_response_variants() {
        let err = TransportErrorKind::custom_str("already known");

        assert!(!err.is_already_known());
        assert!(!err.is_replacement_underpriced());
        assert!(!err.is_transaction_underpriced());
        assert!(!err.is_nonce_too_low());
    }
}
