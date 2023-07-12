use crate::{utils::from_json, RpcObject};

use jsonrpsee_types::ErrorObject;
use std::error::Error as StdError;
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

    #[error(transparent)]
    Custom(Box<dyn StdError + Send + Sync + 'static>),
}

impl TransportError {
    pub fn ser_err(err: serde_json::Error) -> Self {
        Self::SerdeJson { err, text: None }
    }

    pub fn deser_err(err: serde_json::Error, text: impl AsRef<str>) -> Self {
        Self::from((err, text))
    }

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

#[derive(Error, Debug)]
pub enum RpcResult<T, E: StdError> {
    Ok(T),
    ErrResp(ErrorObject<'static>),
    Err(E),
}

impl<T> From<crate::common::RpcOutcome> for RpcResult<T, TransportError>
where
    T: RpcObject,
{
    fn from(value: crate::common::RpcOutcome) -> Self {
        match value {
            Ok(Ok(val)) => {
                let val = val.get();
                match from_json(val) {
                    Ok(val) => RpcResult::Ok(val),
                    Err(err) => RpcResult::Err(err),
                }
            }
            Ok(Err(err)) => RpcResult::ErrResp(err),
            Err(e) => RpcResult::Err(e),
        }
    }
}
