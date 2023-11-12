mod error;
pub use error::{BorrowedErrorPayload, ErrorPayload};

mod payload;
pub use payload::{BorrowedResponsePayload, ResponsePayload};

use std::{borrow::Borrow, fmt, marker::PhantomData};

use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;

use crate::{common::Id, RpcResult};
/// A JSONRPC-2.0 response object containing a [`ResponsePayload`].
///
/// This object is used to represent a JSONRPC-2.0 response. It may contain
/// either a successful result or an error. The `id` field is used to match
/// the response to the request that it is responding to, and should be
/// mirrored from the response.
#[derive(Debug, Clone)]
pub struct Response<Payload = Box<RawValue>, ErrData = Box<RawValue>> {
    /// The ID of the request that this response is responding to.
    pub id: Id,
    /// The response payload.
    pub payload: ResponsePayload<Payload, ErrData>,
}

impl<Payload, ErrData, E> From<Response<Payload, ErrData>> for RpcResult<Payload, ErrData, E> {
    fn from(value: Response<Payload, ErrData>) -> Self {
        match value.payload {
            ResponsePayload::Ok(payload) => Ok(Ok(payload)),
            ResponsePayload::Err(payload) => Ok(Err(payload)),
        }
    }
}

/// A [`Response`] that has been partially deserialized, borrowing its contents
/// from the deserializer. This is used primarily for intermediate
/// deserialization. Most users will not require it.
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type BorrowedResponse<'a> = Response<&'a RawValue, &'a RawValue>;

impl BorrowedResponse<'_> {
    /// Convert this borrowed response to an owned response by copying the data
    /// from the deserializer (if necessary).
    pub fn to_owned(&self) -> Response {
        let payload = self
            .payload
            .as_deref()
            .map(|r| r.to_owned())
            .map_err(|e| e.to_owned());

        Response {
            id: self.id.clone(),
            payload,
        }
    }
}

impl<Payload, ErrData> Response<Payload, ErrData> {
    /// Returns `true` if the response is a success.
    pub const fn is_ok(&self) -> bool {
        self.payload.is_ok()
    }

    /// Returns `true` if the response is an error.
    pub const fn is_err(&self) -> bool {
        self.payload.is_err()
    }

    /// Fallible conversion to the succesful payload.
    pub const fn as_ok(&self) -> Option<&Payload> {
        match self.payload {
            ResponsePayload::Ok(ref payload) => Some(payload),
            _ => None,
        }
    }

    /// Fallible conversion to the error object.
    pub const fn as_err(&self) -> Option<&ErrorPayload<ErrData>> {
        match self.payload {
            ResponsePayload::Err(ref payload) => Some(payload),
            _ => None,
        }
    }
}

impl<'a, Payload, ErrData> Response<Payload, ErrData>
where
    Payload: AsRef<RawValue> + 'a,
{
    /// Attempt to deserialize the success payload, borrowing from the payload
    /// if necessary.
    ///
    /// ## Returns
    /// - `Some(Ok(T))` if the payload is a success and can be deserialized as
    ///   `T`.
    /// - `Some(Err(err))` if the payload is a success and can't be
    ///   deserialized as `T`
    /// - `None` if the payload is an error response
    pub fn try_success_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        match &self.payload {
            Ok(val) => Some(serde_json::from_str(val.as_ref().get())),
            Err(_) => None,
        }
    }

    /// Attempt to deserialize the Success payload, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(Response<T, ErrData>)` if the payload is a success and can be
    ///   deserialized as T, or if the payload is an error.
    /// - `Err(self)` if the payload is a success and can't be deserialized.
    pub fn deserialize_success<T: DeserializeOwned>(self) -> Result<Response<T, ErrData>, Self> {
        if self.is_ok() {
            let val = self.try_success_as().unwrap();
            match val {
                Ok(val) => {
                    return Ok(Response {
                        id: self.id,
                        payload: ResponsePayload::Ok(val),
                    })
                }
                Err(_) => return Err(self),
            }
        }

        let Response {
            id,
            payload: ResponsePayload::Err(payload),
        } = self
        else {
            unreachable!()
        };

        Ok(Response {
            id,
            payload: ResponsePayload::Err(payload),
        })
    }
}

impl<'a, Payload, ErrData> Response<Payload, ErrData>
where
    ErrData: Borrow<RawValue> + 'a,
{
    /// Attempt to deserialize the error payload, borrowing from the payload if
    /// necesary.
    ///
    /// See [`ErrorPayload::try_data_as`].
    pub fn try_error_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        match &self.payload {
            Ok(_) => None,
            Err(val) => val.try_data_as(),
        }
    }

    /// Attempt to deserialize the Error payload, transforming this type.
    ///
    /// # Returns
    ///
    /// - `Ok(Response<Payload, T>)` if the payload is an error and can be
    ///   deserialized as `T`, or if the payload is a success.
    /// - `Err(self)` if the payload is an error and can't be deserialized.
    pub fn deser_err<T: DeserializeOwned>(self) -> Result<Response<Payload, T>, Self> {
        if self.is_err() {
            let val = self.try_error_as().unwrap();
            match val {
                Ok(val) => {
                    return Ok(Response {
                        id: self.id,
                        payload: ResponsePayload::Err(val),
                    })
                }
                Err(_) => return Err(self),
            }
        }
        let Response {
            id,
            payload: ResponsePayload::Ok(val),
        } = self
        else {
            unreachable!()
        };
        Ok(Response {
            id,
            payload: ResponsePayload::Ok(val),
        })
    }
}

impl<'de, Payload, ErrData> Deserialize<'de> for Response<Payload, ErrData>
where
    Payload: Deserialize<'de>,
    ErrData: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Result,
            Error,
            Id,
            Unknown,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("`result`, `error` and `id`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "result" => Ok(Field::Result),
                            "error" => Ok(Field::Error),
                            "id" => Ok(Field::Id),
                            _ => Ok(Field::Unknown),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct JsonRpcResponseVisitor<T>(PhantomData<T>);

        impl<'de, Payload, ErrData> Visitor<'de> for JsonRpcResponseVisitor<fn() -> (Payload, ErrData)>
        where
            Payload: Deserialize<'de>,
            ErrData: Deserialize<'de>,
        {
            type Value = Response<Payload, ErrData>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(
                    "a JSON-RPC response object, consisting of either a result or an error",
                )
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut result = None;
                let mut error = None;
                let mut id: Option<Id> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Result => {
                            if result.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result = Some(map.next_value()?);
                        }
                        Field::Error => {
                            if error.is_some() {
                                return Err(serde::de::Error::duplicate_field("error"));
                            }
                            error = Some(map.next_value()?);
                        }
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::Unknown => {
                            let _: serde::de::IgnoredAny = map.next_value()?; // ignore
                        }
                    }
                }
                let id = id.unwrap_or(Id::None);

                match (result, error) {
                    (Some(result), None) => Ok(Response {
                        id,
                        payload: ResponsePayload::Ok(result),
                    }),
                    (None, Some(error)) => Ok(Response {
                        id,
                        payload: ResponsePayload::Err(error),
                    }),
                    (None, None) => Err(serde::de::Error::missing_field("result or error")),
                    (Some(_), Some(_)) => Err(serde::de::Error::custom(
                        "result and error are mutually exclusive",
                    )),
                }
            }
        }

        deserializer.deserialize_map(JsonRpcResponseVisitor(PhantomData))
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn deser_success() {
        let response = r#"{
            "jsonrpc": "2.0",
            "result": "california",
            "id": 1
        }"#;
        let response: super::Response = serde_json::from_str(response).unwrap();
        assert_eq!(response.id, super::Id::Number(1));
        assert!(matches!(response.payload, super::ResponsePayload::Ok(_)));
    }

    #[test]
    fn deser_err() {
        let response = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": null
        }"#;
        let response: super::Response = serde_json::from_str(response).unwrap();
        assert_eq!(response.id, super::Id::None);
        assert!(matches!(response.payload, super::ResponsePayload::Err(_)));
    }

    #[test]
    fn deser_complex_success() {
        let response = r#"{
            "result": {
                "name": "california",
                "population": 39250000,
                "cities": [
                    "los angeles",
                    "san francisco"
                ]
            }
        }"#;
        let response: super::Response = serde_json::from_str(response).unwrap();
        assert_eq!(response.id, super::Id::None);
        assert!(matches!(response.payload, super::ResponsePayload::Ok(_)));
    }
}

// Copyright 2019-2021 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
