use std::fmt::Debug;

use serde_json::value::RawValue;

use crate::{ErrorPayload, Response, ResponsePayload, RpcReturn};

/// The result of a JSON-RPC request.
///
/// Either a success response, an error response, or a non-response error. The
/// non-response error is intended to be used for errors returned by a
/// transport, or serde errors.
///
/// The three cases
#[must_use = "Results must be handled."]
#[derive(Debug)]
pub enum RpcResult<T, E> {
    /// Server returned a response.
    Ok(T),
    /// Server returned an error response. No communication or serialization
    /// errors occurred.
    ErrResp(ErrorPayload),
    /// Some other error occurred. This could indicate a transport error, a
    /// serde error, or anything else.
    Err(E),
}

impl<T, E> RpcResult<T, E> {
    /// `true` if the result is an `Ok` value.
    pub fn is_ok(&self) -> bool {
        matches!(self, RpcResult::Ok(_))
    }

    /// `true` if the result is an `ErrResp` value.
    pub fn is_err_resp(&self) -> bool {
        matches!(self, RpcResult::ErrResp(_))
    }

    /// `true` if the result is an `Err` value.
    pub fn is_err(&self) -> bool {
        matches!(self, RpcResult::Err(_))
    }

    /// Unwrap the inner value if it is `Ok`, panic otherwise.
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

    /// Unwrap the inner value if it is `ErrResp`, panic otherwise.
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

    /// Unwrap the inner value if it is `Err`, panic otherwise.
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

    /// Apply `op` to the inner value if it is `Ok`.
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

    /// Calls `op` if the result is `Ok`, otherwise returns the `Err` or
    /// `ErrResp` value of `self`
    pub fn and_then<U, F>(self, op: F) -> RpcResult<U, E>
    where
        F: FnOnce(T) -> RpcResult<U, E>,
    {
        match self {
            RpcResult::Ok(val) => op(val),
            RpcResult::ErrResp(err) => RpcResult::ErrResp(err),
            RpcResult::Err(err) => RpcResult::Err(err),
        }
    }

    /// Apply `op` to the inner value if it is `Err`.
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

    /// Shortcut for `map_err(Into::into)`. Useful for converting between error
    /// types.
    pub fn convert_err<U>(self) -> RpcResult<T, U>
    where
        U: From<E>,
    {
        self.map_err(Into::into)
    }

    /// Drop the inner value if it is `Ok`, returning `()` instead. Used when
    /// we only want success/failure status, and don't care about the response
    /// value.
    pub fn empty(self) -> RpcResult<(), E> {
        self.map(|_| ())
    }

    /// Converts from `RpcResult<T, E>` to `Option<T>`.
    pub fn ok(self) -> Option<T> {
        match self {
            RpcResult::Ok(val) => Some(val),
            _ => None,
        }
    }

    /// Converts from `RpcResult<T, E>` to `Option<ErrorPayload>`.
    pub fn err_resp(self) -> Option<ErrorPayload> {
        match self {
            RpcResult::ErrResp(err) => Some(err),
            _ => None,
        }
    }

    /// Converts from `RpcResult<T, E>` to `Option<E>`.
    pub fn err(self) -> Option<E> {
        match self {
            RpcResult::Err(err) => Some(err),
            _ => None,
        }
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

impl<E> From<Response> for RpcResult<Box<RawValue>, E> {
    fn from(value: Response) -> Self {
        match value.payload {
            ResponsePayload::Success(res) => Self::Ok(res),
            ResponsePayload::Error(e) => Self::ErrResp(e),
        }
    }
}

impl<E> From<Result<Response, E>> for RpcResult<Box<RawValue>, E> {
    fn from(value: Result<Response, E>) -> Self {
        match value {
            Ok(res) => res.into(),
            Err(err) => Self::Err(err),
        }
    }
}
