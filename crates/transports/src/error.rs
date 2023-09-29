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
        #[source]
        err: serde_json::Error,
        text: Option<String>,
    },

    /// Missing batch response
    #[error("Missing response in batch request")]
    MissingBatchResponse,

    /// Custom error
    #[error(transparent)]
    Custom(Box<dyn StdError + Send + Sync + 'static>),

    /// Hyper http transport.
    #[error(transparent)]
    #[cfg(feature = "reqwest")]
    Reqwest(#[from] reqwest::Error),

    /// Hyper http transport.
    #[error(transparent)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "hyper"))]
    Hyper(#[from] hyper::Error),

    /// Tungstenite websocket transport.
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),

    /// PubSub backend connection task has stopped.
    #[error("PubSub backend connection task has stopped.")]
    BackendGone,
}

impl TransportError {
    /// Instantiate a new `TransportError` from a [`serde_json::Error`]. This
    /// should be called when the error occurs during serialization.
    pub fn ser_err(err: serde_json::Error) -> Self {
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
        Self::SerdeJson {
            err,
            text: Some(text.as_ref().to_string()),
        }
    }
}
