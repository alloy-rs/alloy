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
    serializer.collect_str(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct NumericString(#[serde(with = "crate::u256_numeric_string")] U256);

    #[test]
    fn round_trip_values_above_u128_max() {
        for value in [U256::from(u128::MAX) + U256::from(1u8), U256::MAX] {
            let json = format!("\"{value}\"");
            let deserialized: NumericString = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, NumericString(value));
            assert_eq!(serde_json::to_string(&deserialized).unwrap(), json);
        }
    }
}
