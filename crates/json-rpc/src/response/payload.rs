use std::borrow::Borrow;

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;

use crate::ErrorPayload;

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
#[derive(Debug, Clone)]
pub enum ResponsePayload<Payload = Box<RawValue>, ErrData = Box<RawValue>> {
    Success(Payload),
    Error(ErrorPayload<ErrData>),
}

/// A [`ResponsePayload`] that has been partially deserialized, borrowing its
/// contents from the deserializer. This is used primarily for intermediate
/// deserialization. Most users will not require it.
pub type BorrowedResponsePayload<'a> = ResponsePayload<&'a RawValue, &'a RawValue>;

impl<Payload, ErrData> ResponsePayload<Payload, ErrData> {
    /// Fallible conversion to the succesful payload.
    pub fn as_success(&self) -> Option<&Payload> {
        match self {
            ResponsePayload::Success(payload) => Some(payload),
            _ => None,
        }
    }

    /// Fallible conversion to the error object.
    pub fn as_error(&self) -> Option<&ErrorPayload<ErrData>> {
        match self {
            ResponsePayload::Error(payload) => Some(payload),
            _ => None,
        }
    }

    /// Returns `true` if the response payload is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, ResponsePayload::Success(_))
    }

    /// Returns `true` if the response payload is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, ResponsePayload::Error(_))
    }
}

impl<'a, Payload, ErrData> ResponsePayload<Payload, ErrData>
where
    Payload: AsRef<RawValue> + 'a,
{
    /// Attempt to deserialize the success payload.
    ///
    /// # Returns
    /// - `None` if the payload is an error
    /// - `Some(Ok(T))` if the payload is a success and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is a success and can't
    ///   be deserialized as `T`
    pub fn try_success_as<T: DeserializeOwned>(&self) -> Option<serde_json::Result<T>> {
        self.as_success()
            .map(|payload| serde_json::from_str(payload.as_ref().get()))
    }

    /// Attempt to deserialize the success payload, borrowing from the payload.
    ///
    /// # Returns
    /// - `None` if the payload is an error
    /// - `Some(Ok(T))` if the payload is a success and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is a success and can't
    ///   be deserialized as `T`
    pub fn try_borrow_success_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        self.as_success()
            .map(|payload| serde_json::from_str(payload.as_ref().get()))
    }

    /// Deserialize a Success payload, if possible, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(ResponsePayload<T>)` if the payload is an error, or if the
    ///   payload is a success and can be deserialized as `T`
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
            ResponsePayload::Error(e) => Ok(ResponsePayload::Error(e)),
        }
    }
}

impl<'a, Payload, Data> ResponsePayload<Payload, Data>
where
    Data: Borrow<RawValue> + 'a,
{
    /// Attempt to deserialize the error payload.
    ///
    /// # Returns
    /// - `None` if the payload is a success
    /// - `Some(Ok(T))` if the payload is an error and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is an error and can't
    ///   be deserialized as `T`
    pub fn try_error_as<T: DeserializeOwned>(&self) -> Option<serde_json::Result<T>> {
        self.as_error().and_then(|error| error.try_data_as::<T>())
    }

    /// Attempt to deserialize the error payload, borrowing from the payload.
    ///
    /// # Returns
    /// - `None` if the payload is a success
    /// - `Some(Ok(T))` if the payload is an error and can be deserialized
    /// - `Some(Err(serde_json::Error))` if the payload is an error and can't
    ///   be deserialized as `T`
    pub fn try_borrow_error_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        self.as_error()
            .and_then(|error| error.try_borrow_data_as::<T>())
    }

    /// Deserialize an Error payload, if possible, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(ResponsePayload<Payload, T>)` if the payload is an error, or if
    ///   the payload is an error and can be deserialized as `T`.
    /// - `Err(self)` if the payload is an error and can't be deserialized.
    pub fn deserialize_error<T: DeserializeOwned>(
        self,
    ) -> Result<ResponsePayload<Payload, T>, Self> {
        match self {
            ResponsePayload::Error(err) => match err.deser_data() {
                Ok(deser) => Ok(ResponsePayload::Error(deser)),
                Err(err) => Err(ResponsePayload::Error(err)),
            },
            ResponsePayload::Success(payload) => Ok(ResponsePayload::Success(payload)),
        }
    }
}
