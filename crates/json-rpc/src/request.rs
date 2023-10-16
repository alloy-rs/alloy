use crate::{common::Id, RpcParam};

use serde::{ser::SerializeMap, Deserialize, Serialize};
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
pub struct Request<Params = Box<RawValue>> {
    pub method: &'static str,
    pub params: Params,
    pub id: Id,
}

impl<Params> Request<Params>
where
    Params: RpcParam,
{
    /// Serialize the request parameters as a boxed [`RawValue`].
    ///
    /// # Panics
    ///
    /// If serialization of the params fails.
    pub fn box_params(self) -> Request<Box<RawValue>> {
        Request {
            method: self.method,
            params: RawValue::from_string(serde_json::to_string(&self.params).unwrap()).unwrap(),
            id: self.id,
        }
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
