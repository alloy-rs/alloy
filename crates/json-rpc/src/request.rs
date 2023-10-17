use crate::{common::Id, RpcParam};

use serde::{de::DeserializeOwned, ser::SerializeMap, Deserialize, Serialize};
use serde_json::value::RawValue;

/// A JSON-RPC 2.0 request object.
///
/// This is a generic type that can be used to represent any JSON-RPC request.
/// The `Params` type parameter is used to represent the parameters of the
/// request, and the `method` field is used to represent the method name.
///
/// ### Note
///
/// The value of `method` should be known at compile time.
#[derive(Debug, Deserialize, Clone)]
pub struct Request<Params> {
    pub method: &'static str,
    pub params: Params,
    pub id: Id,
}

/// A [`Request`] that has been partially serialized. The request parameters
/// have been serialized, and are represented as a boxed [`RawValue`]. This is
/// useful for collections containing many requests, as it erases the `Param`
/// type. It can be created with [`Request::box_params()`].
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type ParitallySerializedRequest = Request<Box<RawValue>>;

impl<Params> Request<Params>
where
    Params: RpcParam,
{
    /// Serialize the request parameters as a boxed [`RawValue`].
    ///
    /// # Panics
    ///
    /// If serialization of the params fails.
    pub fn box_params(self) -> ParitallySerializedRequest {
        Request {
            method: self.method,
            params: RawValue::from_string(serde_json::to_string(&self.params).unwrap()).unwrap(),
            id: self.id,
        }
    }
}

impl<'a, Params> Request<Params>
where
    Params: AsRef<RawValue> + 'a,
{
    /// Attempt to deserialize the params.
    ///
    /// To borrow from the params via the deserializer, use
    /// [`Request::try_borrow_params_as`].
    ///
    /// # Returns
    /// - `Ok(T)` if the params can be deserialized as `T`
    /// - `Err(e)` if the params cannot be deserialized as `T`
    pub fn try_params_as<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(self.params.as_ref().get())
    }

    /// Attempt to deserialize the params, borrowing from the params
    ///
    /// # Returns
    /// - `Ok(T)` if the params can be deserialized as `T`
    /// - `Err(e)` if the params cannot be deserialized as `T`
    pub fn try_borrow_params_as<T: Deserialize<'a>>(&'a self) -> serde_json::Result<T> {
        serde_json::from_str(self.params.as_ref().get())
    }
}

// manually implemented to avoid adding a type for the protocol-required
// `jsonrpc` field
impl<Params> Serialize for Request<Params>
where
    Params: RpcParam,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let sized_params = std::mem::size_of::<Params>() != 0;

        let mut map = serializer.serialize_map(Some(3 + sized_params as usize))?;
        map.serialize_entry("method", self.method)?;

        // Params may be omitted if it is 0-sized
        if sized_params {
            // TODO: remove unwrap
            map.serialize_entry("params", &self.params)?;
        }

        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("jsonrpc", "2.0")?;
        map.end()
    }
}
