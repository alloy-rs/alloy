use std::{error::Error as StdError, fmt::Debug};
use thiserror::Error;

/// Transport error.
///
/// All transport errors are wrapped in this enum.
#[derive(Error, Debug)]
pub enum TransportError {
    /// SerdeJson (de)ser
    #[error("{err}")]
    SerdeJson {
        /// The underlying serde_json error.
        #[source]
        err: serde_json::Error,
        /// For deser errors, the text that failed to deserialize.
        text: Option<String>,
    },

    /// Missing batch response
    #[error("Missing response in batch request")]
    MissingBatchResponse,

    /// PubSub backend connection task has stopped.
    #[error("PubSub backend connection task has stopped.")]
    BackendGone,

    /// Custom error
    #[error(transparent)]
    Custom(Box<dyn StdError + Send + Sync + 'static>),
}

impl TransportError {
    /// Instantiate a new `TransportError` from a [`serde_json::Error`]. This
    /// should be called when the error occurs during serialization.
    pub const fn ser_err(err: serde_json::Error) -> Self {
        Self::SerdeJson { err, text: None }
    }

    /// Instantiate a new `TransportError` from a [`serde_json::Error`] and the
    /// text. This should be called when the error occurs during
    /// deserialization.
    pub fn deser_err(err: serde_json::Error, text: impl AsRef<str>) -> Self {
        Self::from((err, text))
    }

    /// Instantiate a new `TransportError` from a custom error.
    pub fn custom(err: impl StdError + Send + Sync + 'static) -> Self {
        Self::Custom(Box::new(err))
    }
}

impl<T> From<(serde_json::Error, T)> for TransportError
where
    T: AsRef<str>,
{
    fn from((err, text): (serde_json::Error, T)) -> Self {
        Self::SerdeJson { err, text: Some(text.as_ref().to_string()) }
    }
}
