use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    /// SerdeJson (de)ser
    #[error("{err}")]
    SerdeJson {
        err: serde_json::Error,
        text: String,
    },

    /// Http transport
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl TransportError {
    pub fn ser_err(err: serde_json::Error) -> Self {
        Self::SerdeJson {
            err,
            text: "".to_string(),
        }
    }

    pub fn deser_err(err: serde_json::Error, text: impl AsRef<str>) -> Self {
        Self::SerdeJson {
            err,
            text: text.as_ref().to_string(),
        }
    }
}
