//! Numeric serde helpers.

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

/// serde functions for handling `u8` via [U8](alloy_primitives::U8)
pub mod u8_via_ruint {
    use alloy_primitives::U8;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u8` from [U8] accepting a hex quantity string with optional 0x prefix
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u8, D::Error>
    where
        D: Deserializer<'de>,
    {
        U8::deserialize(deserializer).map(|val| val.to())
    }

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &u8, s: S) -> Result<S::Ok, S::Error> {
        U8::from(*value).serialize(s)
    }
}

/// serde functions for handling `Option<u8>` via [U8](alloy_primitives::U8)
pub mod u8_opt_via_ruint {
    use alloy_primitives::U8;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &Option<u8>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(val) => U8::from(*val).serialize(s),
            None => s.serialize_none(),
        }
    }

    /// Deserializes an `Option` from [U8] accepting a hex quantity string with optional 0x prefix
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(U8::deserialize(deserializer).map_or(None, |v| Some(u8::from_be_bytes(v.to_be_bytes()))))
    }
}

/// serde functions for handling `u64` via [U64](alloy_primitives::U64)
pub mod u64_via_ruint {
    use alloy_primitives::U64;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u64` from [U64] accepting a hex quantity string with optional 0x prefix
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        U64::deserialize(deserializer).map(|val| val.to())
    }

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        U64::from(*value).serialize(s)
    }
}

/// serde functions for handling `Option<u64>` via [U64](alloy_primitives::U64)
pub mod u64_opt_via_ruint {
    use alloy_primitives::U64;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(val) => U64::from(*val).serialize(s),
            None => s.serialize_none(),
        }
    }

    /// Deserializes an `Option` from [U64] accepting a hex quantity string with optional 0x prefix
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(U64::deserialize(deserializer)
            .map_or(None, |v| Some(u64::from_be_bytes(v.to_be_bytes()))))
    }
}

/// serde functions for handling primitive `u128` via [U128](alloy_primitives::U128)
pub mod u128_via_ruint {
    use alloy_primitives::U128;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u128` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        U128::deserialize(deserializer).map(|val| val.to())
    }

    /// Serializes u128 as hex string
    pub fn serialize<S: Serializer>(value: &u128, s: S) -> Result<S::Ok, S::Error> {
        U128::from(*value).serialize(s)
    }
}

/// serde functions for handling primitive optional `u128` via [U128](alloy_primitives::U128)
pub mod u128_opt_via_ruint {
    use alloy_primitives::U128;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `Option<u128>` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<U128>::deserialize(deserializer)? {
            Some(val) => Ok(Some(val.to())),
            None => Ok(None),
        }
    }

    /// Serializes `Option<u128>` as hex string
    pub fn serialize<S: Serializer>(value: &Option<u128>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(val) => U128::from(*val).serialize(s),
            None => s.serialize_none(),
        }
    }
}

/// serde functions for handling `Vec<u128>` via [U128](alloy_primitives::U128)
pub mod u128_vec_via_ruint {
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;
    use alloy_primitives::U128;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u128` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<U128>::deserialize(deserializer)?;
        Ok(vec.into_iter().map(|val| val.to()).collect())
    }

    /// Serializes u128 as hex string
    pub fn serialize<S: Serializer>(value: &[u128], s: S) -> Result<S::Ok, S::Error> {
        let vec = value.iter().map(|val| U128::from(*val)).collect::<Vec<_>>();
        vec.serialize(s)
    }
}

/// serde functions for handling `Vec<Vec<u128>>` via [U128](alloy_primitives::U128)
pub mod u128_vec_vec_opt_via_ruint {
    use alloy_primitives::U128;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    /// Deserializes an `u128` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Vec<u128>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<Vec<Vec<U128>>>::deserialize(deserializer)? {
            Some(vec) => Ok(Some(
                vec.into_iter().map(|v| v.into_iter().map(|val| val.to()).collect()).collect(),
            )),
            None => Ok(None),
        }
    }

    /// Serializes u128 as hex string
    pub fn serialize<S: Serializer>(
        value: &Option<Vec<Vec<u128>>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(vec) => {
                let vec = vec
                    .iter()
                    .map(|v| v.iter().map(|val| U128::from(*val)).collect::<Vec<_>>())
                    .collect::<Vec<_>>();
                vec.serialize(s)
            }
            None => s.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[cfg(not(feature = "std"))]
    use alloc::{vec, vec::Vec};

    #[test]
    fn test_hex_u64() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "u64_via_ruint")]
            inner: u64,
        }

        let val = Value { inner: 1000 };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "u128_via_ruint")]
            inner: u128,
        }

        let val = Value { inner: 1000 };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let s = "{\"inner\":\"1000\"}".to_string();
        let deserialized: Value = serde_json::from_str(&s).unwrap();

        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_opt_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "u128_opt_via_ruint")]
            inner: Option<u128>,
        }

        let val = Value { inner: Some(1000) };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let s = "{\"inner\":\"1000\"}".to_string();
        let deserialized: Value = serde_json::from_str(&s).unwrap();

        assert_eq!(val, deserialized);

        let val = Value { inner: None };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":null}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_vec_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "u128_vec_via_ruint")]
            inner: Vec<u128>,
        }

        let val = Value { inner: vec![1000, 2000] };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":[\"0x3e8\",\"0x7d0\"]}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_vec_vec_opt_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "u128_vec_vec_opt_via_ruint")]
            inner: Option<Vec<Vec<u128>>>,
        }

        let val = Value { inner: Some(vec![vec![1000, 2000], vec![3000, 4000]]) };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":[[\"0x3e8\",\"0x7d0\"],[\"0xbb8\",\"0xfa0\"]]}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let val = Value { inner: None };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":null}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }
}
