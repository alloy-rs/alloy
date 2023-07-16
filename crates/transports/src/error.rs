use crate::rpc_types::{ErrorPayload, JsonRpcResponse, ResponsePayload, RpcReturn};

use serde_json::value::RawValue;
use std::{error::Error as StdError, fmt::Debug};
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

    /// Missing batch response
    #[error("Missing response in batch request")]
    MissingBatchResponse,

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
#[must_use = "Results must be handled."]
#[derive(Error, Debug)]
pub enum RpcResult<T, E> {
    Ok(T),
    ErrResp(ErrorPayload),
    Err(E),
}

impl<T, E> RpcResult<T, E> {
    pub fn is_ok(&self) -> bool {
        matches!(self, RpcResult::Ok(_))
    }

    pub fn is_err_resp(&self) -> bool {
        matches!(self, RpcResult::ErrResp(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, RpcResult::Err(_))
    }

    pub fn map<U, F>(self, op: F) -> RpcResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            RpcResult::Ok(val) => RpcResult::Ok(op(val)),
            RpcResult::ErrResp(err) => RpcResult::ErrResp(err),
            RpcResult::Err(err) => RpcResult::Err(err),
        }
    }

    pub fn unwrap(self) -> T
    where
        E: Debug,
    {
        match self {
            RpcResult::Ok(val) => val,
            RpcResult::ErrResp(err) => panic!("Error response: {:?}", err),
            RpcResult::Err(err) => panic!("Error: {:?}", err),
        }
    }

    pub fn unwrap_err_resp(self) -> ErrorPayload
    where
        T: Debug,
        E: Debug,
    {
        match self {
            RpcResult::Ok(val) => panic!("Ok: {:?}", val),
            RpcResult::ErrResp(err) => err,
            RpcResult::Err(err) => panic!("Error: {:?}", err),
        }
    }

    pub fn unwrap_err(self) -> E
    where
        T: Debug,
        E: Debug,
    {
        match self {
            RpcResult::Ok(val) => panic!("Ok: {:?}", val),
            RpcResult::ErrResp(err) => panic!("Error response: {:?}", err),
            RpcResult::Err(err) => err,
        }
    }

    pub fn map_err<U, F>(self, op: F) -> RpcResult<T, U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            RpcResult::Ok(val) => RpcResult::Ok(val),
            RpcResult::ErrResp(err) => RpcResult::ErrResp(err),
            RpcResult::Err(err) => RpcResult::Err(op(err)),
        }
    }

    pub fn convert_err<U>(self) -> RpcResult<T, U>
    where
        U: From<E>,
    {
        self.map_err(Into::into)
    }

    pub fn empty(self) -> RpcResult<(), E> {
        self.map(|_| ())
    }
}

impl RpcResult<Box<RawValue>, TransportError> {
    pub fn deser_ok<Resp: RpcReturn>(self) -> RpcResult<Resp, TransportError> {
        match self {
            RpcResult::Ok(val) => match serde_json::from_str(val.get()) {
                Ok(val) => RpcResult::Ok(val),
                Err(err) => RpcResult::Err(TransportError::deser_err(err, val.get())),
            },
            RpcResult::ErrResp(er) => RpcResult::ErrResp(er),
            RpcResult::Err(e) => RpcResult::Err(e),
        }
    }
}

impl<T, E> From<TransportError> for RpcResult<T, E>
where
    E: StdError + From<TransportError>,
{
    fn from(value: TransportError) -> Self {
        RpcResult::Err(value.into())
    }
}

impl From<JsonRpcResponse> for RpcResult<Box<RawValue>, TransportError> {
    fn from(value: JsonRpcResponse) -> Self {
        match value.payload {
            ResponsePayload::Success(res) => RpcResult::Ok(res),
            ResponsePayload::Error(e) => RpcResult::ErrResp(e),
        }
    }
}

impl From<Result<JsonRpcResponse, TransportError>> for RpcResult<Box<RawValue>, TransportError> {
    fn from(value: Result<JsonRpcResponse, TransportError>) -> Self {
        match value {
            Ok(res) => res.into(),
            Err(err) => err.into(),
        }
    }
}
