use crate::{ErrorPayload, Response, ResponsePayload, RpcReturn};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{borrow::Borrow, fmt::Debug};

/// The result of a JSON-RPC request.
///
/// Either a success response, an error response, or a non-response error. The
/// non-response error is intended to be used for errors returned by a
/// transport, or serde errors.
///
/// The three cases are
/// - Success: The server returned a succesful response.
/// - Failure: The server returned an error response.
/// - Err: Some client-side or communication error occurred.
#[must_use = "Results must be handled."]
#[derive(Debug)]
pub enum RpcResult<T, ErrData, E> {
    /// Server returned a succesful response.
    Success(T),
    /// Server returned an error response. No communication or serialization
    /// errors occurred.
    Failure(ErrorPayload<ErrData>),
    /// Some other error occurred. This could indicate a transport error, a
    /// serialization error, or anything else.
    Err(E),
}

/// A [`RpcResult`] that has been partially deserialized, borrowing its
/// contents from the deserializer. This is used primarily for intermediate
/// deserialization. Most users will not require it.
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type BorrowedRpcResult<'a, E> = RpcResult<&'a RawValue, &'a RawValue, E>;

impl<'a, E> BorrowedRpcResult<'a, E> {
    /// Convert this borrowed RpcResult into an owned RpcResult by copying
    /// the data from the deserializer (if necessary).
    pub fn into_owned(self) -> RpcResult<Box<RawValue>, Box<RawValue>, E> {
        match self {
            RpcResult::Success(val) => RpcResult::Success(val.to_owned()),
            RpcResult::Failure(err) => RpcResult::Failure(err.into_owned()),
            RpcResult::Err(err) => RpcResult::Err(err),
        }
    }
}

impl<T, ErrData, E> RpcResult<T, ErrData, E> {
    /// `true` if the result is an `Ok` value.
    pub const fn is_success(&self) -> bool {
        matches!(self, RpcResult::Success(_))
    }

    /// `true` if the result is an `Failure` value.
    pub const fn is_failure(&self) -> bool {
        matches!(self, RpcResult::Failure(_))
    }

    /// `true` if the result is an `Err` value.
    pub const fn is_err(&self) -> bool {
        matches!(self, RpcResult::Err(_))
    }

    /// Unwrap the inner value if it is `Ok`, panic otherwise.
    pub fn unwrap(self) -> T
    where
        ErrData: Debug,
        E: Debug,
    {
        match self {
            RpcResult::Success(val) => val,
            RpcResult::Failure(err) => panic!("Error response: {:?}", err),
            RpcResult::Err(err) => panic!("Error: {:?}", err),
        }
    }

    /// Unwrap the inner value if it is `Failure`, panic otherwise.
    pub fn unwrap_failure(self) -> ErrorPayload<ErrData>
    where
        T: Debug,
        E: Debug,
    {
        match self {
            RpcResult::Success(val) => panic!("Ok: {:?}", val),
            RpcResult::Failure(err) => err,
            RpcResult::Err(err) => panic!("Error: {:?}", err),
        }
    }

    /// Unwrap the inner value if it is `Err`, panic otherwise.
    pub fn unwrap_err(self) -> E
    where
        T: Debug,
        ErrData: Debug,
        E: Debug,
    {
        match self {
            RpcResult::Success(val) => panic!("Ok: {:?}", val),
            RpcResult::Failure(err) => panic!("Error response: {:?}", err),
            RpcResult::Err(err) => err,
        }
    }

    /// Apply `op` to the inner value if it is `Ok`.
    pub fn map<U, F>(self, op: F) -> RpcResult<U, ErrData, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            RpcResult::Success(val) => RpcResult::Success(op(val)),
            RpcResult::Failure(err) => RpcResult::Failure(err),
            RpcResult::Err(err) => RpcResult::Err(err),
        }
    }

    /// Calls `op` if the result is `Ok`, otherwise returns the `Err` or
    /// `Failure` value of `self`
    pub fn and_then<U, F>(self, op: F) -> RpcResult<U, ErrData, E>
    where
        F: FnOnce(T) -> RpcResult<U, ErrData, E>,
    {
        match self {
            RpcResult::Success(val) => op(val),
            RpcResult::Failure(err) => RpcResult::Failure(err),
            RpcResult::Err(err) => RpcResult::Err(err),
        }
    }

    /// Apply `op` to the inner value if it is `Err`.
    pub fn map_err<U, F>(self, op: F) -> RpcResult<T, ErrData, U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            RpcResult::Success(val) => RpcResult::Success(val),
            RpcResult::Failure(err) => RpcResult::Failure(err),
            RpcResult::Err(err) => RpcResult::Err(op(err)),
        }
    }

    /// Shortcut for `map_err(Into::into)`. Useful for converting between error
    /// types.
    pub fn convert_err<U>(self) -> RpcResult<T, ErrData, U>
    where
        U: From<E>,
    {
        self.map_err(Into::into)
    }

    /// Drop the inner value if it is `Ok`, returning `()` instead. Used when
    /// we only want success/failure status, and don't care about the response
    /// value.
    pub fn empty(self) -> RpcResult<(), ErrData, E> {
        self.map(|_| ())
    }

    /// Converts from `RpcResult<T, ErrData, E>` to `Option<T>`.
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn success(self) -> Option<T> {
        match self {
            RpcResult::Success(val) => Some(val),
            _ => None,
        }
    }

    /// Converts from `RpcResult<T, ErrData, E>` to `Option<ErrorPayload>`.
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn failure(self) -> Option<ErrorPayload<ErrData>> {
        match self {
            RpcResult::Failure(err) => Some(err),
            _ => None,
        }
    }

    /// Converts from `RpcResult<T, ErrData, E>` to `Option<E>`.
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn err(self) -> Option<E> {
        match self {
            RpcResult::Err(err) => Some(err),
            _ => None,
        }
    }
}

impl<B, ErrData, E> RpcResult<B, ErrData, E>
where
    B: Borrow<RawValue>,
{
    /// Deserialize a response, if it is `Success`.
    ///
    /// # Returns
    /// - `None` if the response is not `Success`.
    /// - `Some(Ok(Resp))` if the response is `Success` and the
    ///   `result` field can be deserialized.
    /// - `Some(Err(err))` if the response is `Success` and the `result` field
    ///   can't be deserialized.
    pub fn try_success_as<'a, Resp: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<Resp>> {
        match self {
            Self::Success(val) => Some(serde_json::from_str(val.borrow().get())),
            _ => None,
        }
    }

    /// Deserialize the inner value, if it is `Ok`. Pass through other values.
    pub fn deserialize_success<Resp: RpcReturn>(self) -> Result<RpcResult<Resp, ErrData, E>, Self> {
        match self {
            RpcResult::Success(ref ok) => match serde_json::from_str(ok.borrow().get()) {
                Ok(val) => Ok(RpcResult::Success(val)),
                Err(_) => Err(self),
            },
            RpcResult::Failure(err) => Ok(RpcResult::Failure(err)),
            RpcResult::Err(err) => Ok(RpcResult::Err(err)),
        }
    }

    /// Deserialize the inner value, if it is `Ok`. Pass through other values.
    /// Transform deser errors with `F`.
    #[doc(hidden)]
    pub fn try_deserialize_success_or_else<T, F>(self, f: F) -> RpcResult<T, ErrData, E>
    where
        T: RpcReturn,
        F: FnOnce(serde_json::Error, &str) -> E,
    {
        match self {
            RpcResult::Success(val) => {
                let text = val.borrow().get();
                match serde_json::from_str(text) {
                    Ok(val) => RpcResult::Success(val),
                    Err(e) => RpcResult::Err(f(e, text)),
                }
            }
            RpcResult::Failure(f) => RpcResult::Failure(f),
            RpcResult::Err(e) => RpcResult::Err(e),
        }
    }
}

impl<T, B, E> RpcResult<T, B, E>
where
    B: Borrow<RawValue>,
{
    /// Deserialize a response, if it is `Failure`.
    ///
    /// # Returns
    /// - `None` if the response is not `Failure`
    /// - `Some(Ok(ErrorPayload))` if the response is `Failure` and the
    ///   `data` field can be deserialized.
    /// - `Some(Err(err))` if the response is `Failure` and the `data` field
    ///   can't be deserialized.
    pub fn try_failure_as<'a, ErrData: Deserialize<'a>>(
        &'a self,
    ) -> Option<serde_json::Result<ErrData>> {
        match self {
            RpcResult::Failure(err) => err.try_data_as::<ErrData>(),
            _ => None,
        }
    }

    /// Deserialize the inner value, if it is `Failure`. Pass through other
    /// values.
    pub fn deserialize_failure<ErrData: RpcReturn>(self) -> Result<RpcResult<T, ErrData, E>, Self> {
        match self {
            RpcResult::Success(ok) => Ok(RpcResult::Success(ok)),
            RpcResult::Failure(err_resp) => err_resp
                .deser_data::<ErrData>()
                .map(RpcResult::Failure)
                .map_err(RpcResult::Failure),
            RpcResult::Err(err) => Ok(RpcResult::Err(err)),
        }
    }

    /// Deserialize the inner value, if it is `Failure`. Pass through other
    /// values. Transform deser errors with `F`.
    #[doc(hidden)]
    pub fn try_deserialize_failure_or_else<ErrData, F>(
        self,
        f: F,
    ) -> Result<RpcResult<T, ErrData, E>, E>
    where
        ErrData: RpcReturn,
        F: FnOnce(serde_json::Error, &str) -> E,
    {
        match self {
            RpcResult::Success(ok) => Ok(RpcResult::Success(ok)),
            RpcResult::Failure(err_resp) => match err_resp.try_data_as::<ErrData>() {
                None => Ok(RpcResult::Failure(ErrorPayload {
                    code: err_resp.code,
                    message: err_resp.message,
                    data: None,
                })),
                Some(Ok(data)) => Ok(RpcResult::Failure(ErrorPayload {
                    code: err_resp.code,
                    message: err_resp.message,
                    data: Some(data),
                })),
                Some(Err(e)) => {
                    let text = err_resp
                        .data
                        .as_ref()
                        .map(|d| d.borrow().get())
                        .unwrap_or("");
                    Err(f(e, text))
                }
            },

            RpcResult::Err(err) => Ok(RpcResult::Err(err)),
        }
    }
}

impl<Payload, ErrData, E> From<Response<Payload, ErrData>> for RpcResult<Payload, ErrData, E> {
    fn from(value: Response<Payload, ErrData>) -> Self {
        match value.payload {
            ResponsePayload::Success(res) => Self::Success(res),
            ResponsePayload::Failure(e) => Self::Failure(e),
        }
    }
}

impl<Payload, ErrData, E> From<Result<Response<Payload, ErrData>, E>>
    for RpcResult<Payload, ErrData, E>
{
    fn from(value: Result<Response<Payload, ErrData>, E>) -> Self {
        match value {
            Ok(res) => res.into(),
            Err(err) => Self::Err(err),
        }
    }
}
