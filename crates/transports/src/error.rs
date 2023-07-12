use crate::{common::RpcOutcome, utils::from_json, RpcObject};

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

/// The result of a JSON-RPC request. Either a success response, an error
/// response, or another error.
#[derive(Error, Debug)]
pub enum RpcResult<T, E> {
    Ok(T),
    ErrResp(ErrorObject<'static>),
    Err(E),
}

impl<T, E> From<TransportError> for RpcResult<T, E>
where
    E: StdError + From<TransportError>,
{
    fn from(value: TransportError) -> Self {
        RpcResult::Err(value.into())
    }
}

impl<T> From<RpcOutcome> for RpcResult<T, TransportError>
where
    T: RpcObject,
{
    fn from(value: RpcOutcome) -> Self {
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
