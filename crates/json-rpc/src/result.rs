use std::fmt::Debug;

use serde_json::value::RawValue;

use crate::{response::ErrorPayload, JsonRpcResponse, ResponsePayload, RpcReturn};

/// The result of a JSON-RPC request. Either a success response, an error
/// response, or another error.
#[must_use = "Results must be handled."]
#[derive(Debug)]
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

impl<E> RpcResult<Box<RawValue>, E> {
    pub fn deser_ok<Resp: RpcReturn>(self) -> RpcResult<Resp, E>
    where
        E: From<serde_json::Error>,
    {
        match self {
            RpcResult::Ok(val) => match serde_json::from_str(val.get()) {
                Ok(val) => RpcResult::Ok(val),
                Err(err) => RpcResult::Err(err.into()),
            },
            Self::ErrResp(er) => RpcResult::ErrResp(er),
            Self::Err(e) => RpcResult::Err(e),
        }
    }

    #[doc(hidden)]
    pub fn deser_ok_or_else<Resp: RpcReturn, F>(self, f: F) -> RpcResult<Resp, E>
    where
        F: FnOnce(serde_json::Error, &str) -> E,
    {
        match self {
            RpcResult::Ok(val) => match serde_json::from_str(val.get()) {
                Ok(val) => RpcResult::Ok(val),
                Err(err) => RpcResult::Err(f(err, val.get())),
            },
            Self::ErrResp(er) => RpcResult::ErrResp(er),
            Self::Err(e) => RpcResult::Err(e),
        }
    }
}

impl<E> From<JsonRpcResponse> for RpcResult<Box<RawValue>, E> {
    fn from(value: JsonRpcResponse) -> Self {
        match value.payload {
            ResponsePayload::Success(res) => Self::Ok(res),
            ResponsePayload::Error(e) => Self::ErrResp(e),
        }
    }
}

impl<E> From<Result<JsonRpcResponse, E>> for RpcResult<Box<RawValue>, E> {
    fn from(value: Result<JsonRpcResponse, E>) -> Self {
        match value {
            Ok(res) => res.into(),
            Err(err) => Self::Err(err),
        }
    }
}
