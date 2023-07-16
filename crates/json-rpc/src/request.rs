use crate::common::Id;

use serde::{ser::SerializeMap, Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Debug, Deserialize, Clone)]
pub struct JsonRpcRequest {
    pub method: &'static str,
    pub params: Box<RawValue>,
    pub id: Id,
}

impl Serialize for JsonRpcRequest {
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
