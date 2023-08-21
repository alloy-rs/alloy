use crate::{common::Id, RpcParam};

use serde::{ser::SerializeMap, Deserialize, Serialize};

/// A JSON-RPC 2.0 request object.
///
/// This is a generic type that can be used to represent any JSON-RPC request.
/// The `Params` type parameter is used to represent the parameters of the
/// request, and the `method` field is used to represent the method name.
///
/// ### Note
///
/// The value of `method` must be known at compile time.
#[derive(Debug, Deserialize, Clone)]
pub struct JsonRpcRequest<Params> {
    pub method: &'static str,
    pub params: Params,
    pub id: Id,
}

impl<Params> Serialize for JsonRpcRequest<Params>
where
    Params: RpcParam,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("method", self.method)?;
        map.serialize_entry("params", &self.params)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("jsonrpc", "2.0")?;
        map.end()
    }
}
