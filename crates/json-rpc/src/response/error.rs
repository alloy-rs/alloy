use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::value::RawValue;
use std::{borrow::Borrow, fmt, marker::PhantomData};

/// A JSONRPC-2.0 error object.
///
/// This response indicates that the server received and handled the request,
/// but that there was an error in the processing of it. The error should be
/// included in the `message` field of the response payload.
#[derive(Clone, Debug, Serialize)]
pub struct ErrorPayload<ErrData = Box<RawValue>> {
    /// The error code.
    pub code: i64,
    /// The error message (if any).
    pub message: String,
    /// The error data (if any).
    pub data: Option<ErrData>,
}

impl<ErrData> fmt::Display for ErrorPayload<ErrData> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error code {}: {}", self.code, self.message)
    }
}

/// A [`ErrorPayload`] that has been partially deserialized, borrowing its
/// contents from the deserializer. This is used primarily for intermediate
/// deserialization. Most users will not require it.
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type BorrowedErrorPayload<'a> = ErrorPayload<&'a RawValue>;

impl BorrowedErrorPayload<'_> {
    /// Convert this borrowed error payload into an owned payload by copying
    /// the data from the deserializer (if necessary).
    pub fn into_owned(self) -> ErrorPayload {
        ErrorPayload {
            code: self.code,
            message: self.message,
            data: self.data.map(|data| data.to_owned()),
        }
    }
}

impl<'de, ErrData: Deserialize<'de>> Deserialize<'de> for ErrorPayload<ErrData> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Code,
            Message,
            Data,
            Unknown,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("`code`, `message` and `data`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "code" => Ok(Field::Code),
                            "message" => Ok(Field::Message),
                            "data" => Ok(Field::Data),
                            _ => Ok(Field::Unknown),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ErrorPayloadVisitor<T>(PhantomData<T>);

        impl<'de, Data> Visitor<'de> for ErrorPayloadVisitor<Data>
        where
            Data: Deserialize<'de>,
        {
            type Value = ErrorPayload<Data>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a JSON-RPC2.0 error object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut code = None;
                let mut message = None;
                let mut data = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Code => {
                            if code.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code = Some(map.next_value()?);
                        }
                        Field::Message => {
                            if message.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message = Some(map.next_value()?);
                        }
                        Field::Data => {
                            if data.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                        Field::Unknown => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                            // ignore
                        }
                    }
                }
                Ok(ErrorPayload {
                    code: code.ok_or_else(|| serde::de::Error::missing_field("code"))?,
                    message: message.unwrap_or_default(),
                    data,
                })
            }
        }

        deserializer.deserialize_any(ErrorPayloadVisitor(PhantomData))
    }
}

impl<'a, Data> ErrorPayload<Data>
where
    Data: Borrow<RawValue> + 'a,
{
    /// Deserialize the error's `data` field, borrowing from the data field if
    /// necessary.
    ///
    /// # Returns
    ///
    /// - `None` if the error has no `data` field.
    /// - `Some(Ok(data))` if the error has a `data` field that can be deserialized.
    /// - `Some(Err(err))` if the error has a `data` field that can't be deserialized.
    pub fn try_data_as<T: Deserialize<'a>>(&'a self) -> Option<serde_json::Result<T>> {
        self.data.as_ref().map(|data| serde_json::from_str(data.borrow().get()))
    }

    /// Attempt to deserialize the data field.
    ///
    /// # Returns
    ///
    /// - `Ok(ErrorPayload<T>)` if the data field can be deserialized
    /// - `Err(self)` if the data field can't be deserialized, or if there is no data field.
    pub fn deser_data<T: DeserializeOwned>(self) -> Result<ErrorPayload<T>, Self> {
        match self.try_data_as::<T>() {
            Some(Ok(data)) => {
                Ok(ErrorPayload { code: self.code, message: self.message, data: Some(data) })
            }
            _ => Err(self),
        }
    }
}

#[cfg(test)]
mod test {
    use super::BorrowedErrorPayload;
    use crate::ErrorPayload;

    #[test]
    fn smooth_borrowing() {
        let json = r#"{ "code": -32000, "message": "b", "data": null }"#;
        let payload: BorrowedErrorPayload<'_> = serde_json::from_str(json).unwrap();

        assert_eq!(payload.code, -32000);
        assert_eq!(payload.message, "b");
        assert_eq!(payload.data.unwrap().get(), "null");
    }

    #[test]
    fn smooth_deser() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct TestData {
            a: u32,
            b: Option<String>,
        }

        let json = r#"{ "code": -32000, "message": "b", "data": { "a": 5, "b": null } }"#;

        let payload: BorrowedErrorPayload<'_> = serde_json::from_str(json).unwrap();
        let data: TestData = payload.try_data_as().unwrap().unwrap();
        assert_eq!(data, TestData { a: 5, b: None });
    }

    #[test]
    fn missing_data() {
        let json = r#"{"code":-32007,"message":"20/second request limit reached - reduce calls per second or upgrade your account at quicknode.com"}"#;
        let payload: ErrorPayload = serde_json::from_str(json).unwrap();

        assert_eq!(payload.code, -32007);
        assert_eq!(payload.message, "20/second request limit reached - reduce calls per second or upgrade your account at quicknode.com");
        assert!(payload.data.is_none());
    }
}
