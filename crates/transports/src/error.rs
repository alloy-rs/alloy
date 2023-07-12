use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    /// SerdeJson (de)ser
    #[error("{err}")]
    SerdeJson {
        #[source]
        err: serde_json::Error,
        text: Option<String>,
    },

    /// Http transport
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl TransportError {
    pub fn ser_err(err: serde_json::Error) -> Self {
        Self::SerdeJson { err, text: None }
    }

    pub fn deser_err(err: serde_json::Error, text: impl AsRef<str>) -> Self {
        Self::from((err, text))
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
