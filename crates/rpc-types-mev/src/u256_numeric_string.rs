use alloy_primitives::U256;
use serde::{de, Deserialize, Serializer};
use std::str::FromStr;

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: de::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::String(s) => {
            if let Ok(val) = s.parse::<u128>() {
                return Ok(U256::from(val));
            }
            U256::from_str(&s).map_err(de::Error::custom)
        }
        serde_json::Value::Number(num) => {
            num.as_u64().map(U256::from).ok_or_else(|| de::Error::custom("invalid u256"))
        }
        _ => Err(de::Error::custom("invalid u256")),
    }
}

pub(crate) fn serialize<S>(val: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let val: u128 = (*val).try_into().map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&val.to_string())
}
