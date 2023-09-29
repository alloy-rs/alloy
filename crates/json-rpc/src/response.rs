use std::fmt;

use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::value::RawValue;

use crate::common::Id;

/// A JSONRPC-2.0 error object.
///
/// This response indicates that the server received and handled the request,
/// but that there was an error in the processing of it. The error should be
/// included in the `message` field of the response payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorPayload {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<RawValue>>,
}

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
#[derive(Debug, Clone)]
pub enum ResponsePayload {
    Success(Box<RawValue>),
    Error(ErrorPayload),
}

/// A JSONRPC-2.0 response object containing a [`ResponsePayload`].
///
/// This object is used to represent a JSONRPC-2.0 response. It may contain
/// either a successful result or an error. The `id` field is used to match
/// the response to the request that it is responding to, and should be
/// mirrored from the response.
#[derive(Debug, Clone)]
pub struct Response {
    pub id: Id,
    pub payload: ResponsePayload,
}

impl<'de> Deserialize<'de> for Response {
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

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

        struct JsonRpcResponseVisitor;

        impl<'de> Visitor<'de> for JsonRpcResponseVisitor {
            type Value = Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
                        payload: ResponsePayload::Success(result),
                    }),
                    (None, Some(error)) => Ok(Response {
                        id,
                        payload: ResponsePayload::Error(error),
                    }),
                    (None, None) => Err(serde::de::Error::missing_field("result or error")),
                    (Some(_), Some(_)) => Err(serde::de::Error::custom(
                        "result and error are mutually exclusive",
                    )),
                }
            }
        }

        deserializer.deserialize_map(JsonRpcResponseVisitor)
    }
}

#[cfg(test)]
mod test {
    #[test]
    pub fn deser_success() {
        let response = r#"{
            "jsonrpc": "2.0",
            "result": "california",
            "id": 1
        }"#;
        let response: super::Response = serde_json::from_str(response).unwrap();
        assert_eq!(response.id, super::Id::Number(1));
        assert!(matches!(
            response.payload,
            super::ResponsePayload::Success(_)
        ));
    }

    #[test]
    pub fn deser_err() {
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
        assert!(matches!(response.payload, super::ResponsePayload::Error(_)));
    }

    #[test]
    pub fn deser_complex_success() {
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
        assert!(matches!(
            response.payload,
            super::ResponsePayload::Success(_)
        ));
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
