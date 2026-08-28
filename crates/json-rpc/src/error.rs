use crate::{ErrorPayload, RpcRecv};
use alloy_primitives::B256;
use serde_json::value::RawValue;

/// A transport error kind that may carry the raw body of an error response.
///
/// [`RpcError`] is generic over its transport error, which is defined by downstream transport
/// crates and therefore opaque here. This trait is the minimal hook those crates implement so that
/// [`RpcError::error_code`] can look inside error responses that never made it through JSON-RPC
/// decoding, such as a JSON-RPC error served with a non-2xx HTTP status.
pub trait ErrorResponseBody {
    /// Returns the raw body of the error response this error carries, if any.
    fn error_response_body(&self) -> Option<&str>;
}

/// An RPC error.
#[derive(Debug, thiserror::Error)]
pub enum RpcError<E, ErrResp = Box<RawValue>> {
    /// Server returned an error response.
    #[error("server returned an error response: {0}")]
    ErrorResp(ErrorPayload<ErrResp>),

    /// Server returned a null response when a non-null response was expected.
    #[error("server returned a null response when a non-null response was expected")]
    NullResp,

    /// Rpc server returned an unsupported feature.
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(&'static str),

    /// Returned when a local pre-processing step fails. This allows custom
    /// errors from local signers or request pre-processors.
    #[error("local usage error: {0}")]
    LocalUsageError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// JSON serialization error.
    #[error("serialization error: {0}")]
    SerError(
        /// The underlying serde_json error.
        // To avoid accidentally confusing ser and deser errors, we do not use
        // the `#[from]` tag.
        #[source]
        serde_json::Error,
    ),
    /// JSON deserialization error.
    #[error("deserialization error: {err}\n{text}")]
    DeserError {
        /// The underlying serde_json error.
        // To avoid accidentally confusing ser and deser errors, we do not use
        // the `#[from]` tag.
        #[source]
        err: serde_json::Error,
        /// For deser errors, the text that failed to deserialize.
        text: String,
    },

    /// Transport error.
    ///
    /// This variant is used when the error occurs during communication.
    #[error(transparent)]
    Transport(
        /// The underlying transport error.
        #[from]
        E,
    ),
}

impl<E, ErrResp> RpcError<E, ErrResp>
where
    ErrResp: RpcRecv,
{
    /// Instantiate a new `ErrorResp` from an error response.
    pub const fn err_resp(err: ErrorPayload<ErrResp>) -> Self {
        Self::ErrorResp(err)
    }

    /// Instantiate a new `LocalUsageError` from a custom error.
    pub fn local_usage(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::LocalUsageError(err.into())
    }

    /// Instantiate a new `LocalUsageError` from a custom error message.
    pub fn local_usage_str(err: &str) -> Self {
        Self::LocalUsageError(err.into())
    }

    /// Instantiate a new `DeserError` from a [`serde_json::Error`] and the
    /// text. This should be called when the error occurs during
    /// deserialization.
    ///
    /// Note: This will check if the response is actually an [ErrorPayload], if so it will return a
    /// [RpcError::ErrorResp].
    pub fn deser_err(err: serde_json::Error, text: impl AsRef<str>) -> Self {
        let text = text.as_ref();

        // check if the response is actually an `ErrorPayload`
        if let Ok(err) = serde_json::from_str::<ErrorPayload<ErrResp>>(text) {
            return Self::ErrorResp(err);
        }

        Self::DeserError { err, text: text.to_owned() }
    }
}

impl<E, ErrResp> RpcError<E, ErrResp> {
    /// Instantiate a new `SerError` from a [`serde_json::Error`]. This
    /// should be called when the error occurs during serialization.
    pub const fn ser_err(err: serde_json::Error) -> Self {
        Self::SerError(err)
    }

    /// Check if the error is a serialization error.
    pub const fn is_ser_error(&self) -> bool {
        matches!(self, Self::SerError(_))
    }

    /// Check if the error is a deserialization error.
    pub const fn is_deser_error(&self) -> bool {
        matches!(self, Self::DeserError { .. })
    }

    /// Check if the error is a transport error.
    pub const fn is_transport_error(&self) -> bool {
        matches!(self, Self::Transport(_))
    }

    /// Check if the error is an error response.
    pub const fn is_error_resp(&self) -> bool {
        matches!(self, Self::ErrorResp(_))
    }

    /// Check if the error is a null response.
    pub const fn is_null_resp(&self) -> bool {
        matches!(self, Self::NullResp)
    }

    /// Check if the error is an unsupported feature error.
    pub const fn is_unsupported_feature(&self) -> bool {
        matches!(self, Self::UnsupportedFeature(_))
    }

    /// Check if the error is a local usage error.
    pub const fn is_local_usage_error(&self) -> bool {
        matches!(self, Self::LocalUsageError(_))
    }

    /// Fallible conversion to an error response.
    pub const fn as_error_resp(&self) -> Option<&ErrorPayload<ErrResp>> {
        match self {
            Self::ErrorResp(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the transport error if this is a [`RpcError::Transport`]
    pub const fn as_transport_err(&self) -> Option<&E> {
        match self {
            Self::Transport(err) => Some(err),
            _ => None,
        }
    }
}

impl<E: ErrorResponseBody, ErrResp> RpcError<E, ErrResp> {
    /// Returns the JSON-RPC error code this error reports, if any.
    ///
    /// This covers regular JSON-RPC error responses as well as JSON-RPC errors that a provider
    /// buried inside a transport-level error response, for example a `-32601` served with an HTTP
    /// 403. The latter never reach [`RpcError::ErrorResp`], so [`RpcError::as_error_resp`] alone
    /// misses them.
    ///
    /// # Example
    ///
    /// ```rust
    /// use alloy_json_rpc::{ErrorPayload, RpcError};
    ///
    /// # struct NoBody;
    /// # impl alloy_json_rpc::ErrorResponseBody for NoBody {
    /// #     fn error_response_body(&self) -> Option<&str> {
    /// #         None
    /// #     }
    /// # }
    /// let error: RpcError<NoBody> = RpcError::ErrorResp(ErrorPayload::method_not_found());
    /// assert_eq!(error.error_code(), Some(-32601));
    /// ```
    pub fn error_code(&self) -> Option<i64> {
        if let Some(payload) = self.as_error_resp() {
            return Some(payload.code);
        }
        error_code_from_body(self.as_transport_err()?.error_response_body()?)
    }
}

impl<E> RpcError<E, Box<RawValue>> {
    /// Parses the error data field as a hex string of specified length.
    ///
    /// Returns `Some(T)` if the data contains a valid hex string of the expected length.
    fn parse_data<T: std::str::FromStr>(&self) -> Option<T> {
        let error_payload = self.as_error_resp()?;
        let data = error_payload.data.as_ref()?;
        let data_str = data.get().trim_matches('"').trim();
        data_str.parse().ok()
    }

    /// Extracts a transaction hash from the error data field.
    ///
    /// Useful for EIP-7966 `eth_sendRawTransactionSync` errors that return
    /// the transaction hash even when the transaction fails.
    ///
    /// Returns `Some(hash)` if the data contains a valid 32-byte hex string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use alloy_json_rpc::{RpcError, ErrorPayload};
    /// use alloy_primitives::B256;
    ///
    /// // Simulate an EIP-7966 error response
    /// let json = r#"{"code":5,"message":"insufficient funds","data":"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}"#;
    /// let error_payload: ErrorPayload = serde_json::from_str(json).unwrap();
    /// let rpc_error: RpcError<(), _> = RpcError::ErrorResp(error_payload);
    ///
    /// if let Some(tx_hash) = rpc_error.tx_hash_data() {
    ///     println!("Transaction hash: {}", tx_hash);
    /// }
    /// ```
    pub fn tx_hash_data(&self) -> Option<B256> {
        self.parse_data()
    }
}

/// Extracts a JSON-RPC error code from a raw error response body.
///
/// Accepts both a full JSON-RPC error response and a bare error object.
fn error_code_from_body(body: &str) -> Option<i64> {
    // Transports may append human-readable diagnostics after the JSON-RPC body, so parse the first
    // complete JSON value instead of requiring the entire body to be JSON.
    let value =
        serde_json::Deserializer::from_str(body).into_iter::<serde_json::Value>().next()?.ok()?;
    value.get("error").unwrap_or(&value).get("code")?.as_i64()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in transport error carrying an optional raw error-response body.
    struct Body(Option<&'static str>);

    impl ErrorResponseBody for Body {
        fn error_response_body(&self) -> Option<&str> {
            self.0
        }
    }

    fn transport(body: Option<&'static str>) -> RpcError<Body> {
        RpcError::Transport(Body(body))
    }

    #[test]
    fn error_code_from_error_response() {
        let error: RpcError<Body> = RpcError::ErrorResp(ErrorPayload::invalid_request());
        assert_eq!(error.error_code(), Some(-32600));
    }

    #[test]
    fn error_code_from_error_response_body() {
        let nested =
            transport(Some(r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"no method"}}"#));
        assert_eq!(nested.error_code(), Some(-32601));

        let bare = transport(Some(r#"{"code":-32000,"message":"Server error"}"#));
        assert_eq!(bare.error_code(), Some(-32000));
    }

    #[test]
    fn error_code_ignores_trailing_diagnostics() {
        let error = transport(Some(
            "{\"code\":-32000,\"message\":\"Server error\"}\nupstream request failed",
        ));
        assert_eq!(error.error_code(), Some(-32000));
    }

    #[test]
    fn error_code_none_for_unrelated_errors() {
        assert_eq!(transport(None).error_code(), None);
        assert_eq!(transport(Some("not JSON")).error_code(), None);
        assert_eq!(RpcError::<Body>::NullResp.error_code(), None);
    }

    #[test]
    fn error_code_is_callable_generically() {
        fn code<E: ErrorResponseBody, ErrResp>(err: &RpcError<E, ErrResp>) -> Option<i64> {
            err.error_code()
        }

        assert_eq!(code(&transport(Some(r#"{"code":-32602}"#))), Some(-32602));
    }
}
