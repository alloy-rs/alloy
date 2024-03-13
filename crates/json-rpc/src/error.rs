use crate::{ErrorPayload, RpcReturn};
use serde_json::value::RawValue;

/// An RPC error.
#[derive(Debug, thiserror::Error)]
pub enum RpcError<E, ErrResp = Box<RawValue>> {
    /// Server returned an error response.
    #[error("server returned an error response: {0}")]
    ErrorResp(ErrorPayload<ErrResp>),

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
    #[error("deserialization error: {err}")]
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
    ErrResp: RpcReturn,
{
    /// Instantiate a new `TransportError` from an error response.
    pub const fn err_resp(err: ErrorPayload<ErrResp>) -> Self {
        Self::ErrorResp(err)
    }

    /// Instantiate a new `TransportError` from a [`serde_json::Error`] and the
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
    /// Instantiate a new `TransportError` from a [`serde_json::Error`]. This
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

    /// Fallible conversion to an error response.
    pub const fn as_error_resp(&self) -> Option<&ErrorPayload<ErrResp>> {
        match self {
            Self::ErrorResp(err) => Some(err),
            _ => None,
        }
    }
}
