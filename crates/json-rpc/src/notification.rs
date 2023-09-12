use alloy_primitives::U256;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

#[derive(Debug, Deserialize)]
pub struct EthNotification<T = Box<serde_json::value::RawValue>> {
    pub subscription: U256,
    pub result: T,
}

impl<T> Serialize for EthNotification<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("subscription", &self.subscription)?;
        state.serialize_entry("result", &self.result)?;
        state.end()
    }
}
