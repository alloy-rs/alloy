use crate::ErrorPayload;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;
use std::borrow::Borrow;

/// A JSONRPC-2.0 response payload.
///
/// This enum covers both the success and error cases of a JSONRPC-2.0
/// response. It is used to represent the `result` and `error` fields of a
/// response object.
///
/// ### Note
///
/// This type does not implement `Serialize` or `Deserialize` directly. It is
/// deserialized as part of the [`Response`] type.
///
/// [`Response`]: crate::Response
#[derive(Clone, Debug)]
pub enum ResponsePayload<Payload = Box<RawValue>, ErrData = Box<RawValue>> {
    /// A successful response payload.
    Success(Payload),
    /// An error response payload.
    Failure(ErrorPayload<ErrData>),
}

/// A [`ResponsePayload`] that has been partially deserialized, borrowing its
/// contents from the deserializer. This is used primarily for intermediate
/// deserialization. Most users will not require it.
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type BorrowedResponsePayload<'a> = ResponsePayload<&'a RawValue, &'a RawValue>;

impl BorrowedResponsePayload<'_> {
    /// Convert this borrowed response payload into an owned payload by copying
    /// the data from the deserializer (if necessary).
    pub fn into_owned(self) -> ResponsePayload {
        match self {
            Self::Success(payload) => ResponsePayload::Success(payload.to_owned()),
            Self::Failure(error) => ResponsePayload::Failure(error.into_owned()),
        }
    }
}

impl<Payload, ErrData> ResponsePayload<Payload, ErrData> {
    /// Fallible conversion to the successful payload.
    pub const fn as_success(&self) -> Option<&Payload> {
        match self {
            ResponsePayload::Success(payload) => Some(payload),
            _ => None,
        }
    }

    /// Fallible conversion to the error object.
    pub const fn as_error(&self) -> Option<&ErrorPayload<ErrData>> {
        match self {
            ResponsePayload::Failure(payload) => Some(payload),
            _ => None,
        }
    }

    /// Returns `true` if the response payload is a success.
    pub const fn is_success(&self) -> bool {
        matches!(self, ResponsePayload::Success(_))
    }

    /// Returns `true` if the response payload is an error.
    pub const fn is_error(&self) -> bool {
        matches!(self, ResponsePayload::Failure(_))
    }
}

impl<'a, Payload, ErrData> ResponsePayload<Payload, ErrData>
where
    Payload: AsRef<RawValue> + 'a,
{
    /// Attempt to deserialize the success payload, borrowing from the payload
    /// if necessary.
    ///
    /// # Returns
    /// - `None` if the payload is an error
    /// - `Some(Ok(T))` if the payload is a success and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is a success and can't be deserialized as
    ///   `T`
    pub fn try_success_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        self.as_success().map(|payload| serde_json::from_str(payload.as_ref().get()))
    }

    /// Deserialize a Success payload, if possible, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(ResponsePayload<T>)` if the payload is an error, or if the payload is a success and
    ///   can be deserialized as `T`
    /// - `Err(self)` if the payload is a success and can't be deserialized
    pub fn deserialize_success<T: DeserializeOwned>(
        self,
    ) -> Result<ResponsePayload<T, ErrData>, Self> {
        match self {
            ResponsePayload::Success(ref payload) => {
                match serde_json::from_str(payload.as_ref().get()) {
                    Ok(payload) => Ok(ResponsePayload::Success(payload)),
                    Err(_) => Err(self),
                }
            }
            ResponsePayload::Failure(e) => Ok(ResponsePayload::Failure(e)),
        }
    }
}

impl<'a, Payload, Data> ResponsePayload<Payload, Data>
where
    Data: Borrow<RawValue> + 'a,
{
    /// Attempt to deserialize the error payload, borrowing from the payload if
    /// necessary.
    ///
    /// # Returns
    /// - `None` if the payload is a success
    /// - `Some(Ok(T))` if the payload is an error and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is an error and can't be deserialized as `T`
    pub fn try_error_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        self.as_error().and_then(|error| error.try_data_as::<T>())
    }

    /// Deserialize an Error payload, if possible, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(ResponsePayload<Payload, T>)` if the payload is an error, or if the payload is an
    ///   error and can be deserialized as `T`.
    /// - `Err(self)` if the payload is an error and can't be deserialized.
    pub fn deserialize_error<T: DeserializeOwned>(
        self,
    ) -> Result<ResponsePayload<Payload, T>, Self> {
        match self {
            ResponsePayload::Failure(err) => match err.deser_data() {
                Ok(deser) => Ok(ResponsePayload::Failure(deser)),
                Err(err) => Err(ResponsePayload::Failure(err)),
            },
            ResponsePayload::Success(payload) => Ok(ResponsePayload::Success(payload)),
        }
    }
}
