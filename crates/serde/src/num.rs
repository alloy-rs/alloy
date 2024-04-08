//! Numeric serde helpers.

#[cfg(not(feature = "std"))]
use alloc::string::ToString;
use core::str::FromStr;

use alloy_primitives::{U256, U64};
use serde::{de, Deserialize, Deserializer, Serialize};

/// A `u64` wrapper type that deserializes from hex or a u64 and serializes as hex.
///
///
/// ```rust
/// use alloy_serde::num::U64HexOrNumber;
/// let number_json = "100";
/// let hex_json = "\"0x64\"";
///
/// let number: U64HexOrNumber = serde_json::from_str(number_json).unwrap();
/// let hex: U64HexOrNumber = serde_json::from_str(hex_json).unwrap();
/// assert_eq!(number, hex);
/// assert_eq!(hex.to(), 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct U64HexOrNumber(U64);

impl U64HexOrNumber {
    /// Returns the wrapped u64
    pub fn to(self) -> u64 {
        self.0.to()
    }
}

impl From<u64> for U64HexOrNumber {
    fn from(value: u64) -> Self {
        Self(U64::from(value))
    }
}

impl From<U64> for U64HexOrNumber {
    fn from(value: U64) -> Self {
        Self(value)
    }
}

impl From<U64HexOrNumber> for u64 {
    fn from(value: U64HexOrNumber) -> Self {
        value.to()
    }
}

impl From<U64HexOrNumber> for U64 {
    fn from(value: U64HexOrNumber) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for U64HexOrNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum NumberOrHexU64 {
            Hex(U64),
            Int(u64),
        }
        match NumberOrHexU64::deserialize(deserializer)? {
            NumberOrHexU64::Int(val) => Ok(val.into()),
            NumberOrHexU64::Hex(val) => Ok(val.into()),
        }
    }
}

/// serde functions for handling `u8` as [U8](alloy_primitives::U8)
pub mod u8_hex {
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

/// serde functions for handling `Option<u8>` as [U8](alloy_primitives::U8)
pub mod u8_hex_opt {
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

/// serde functions for handling `u64` as [U64]
pub mod u64_hex {
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

/// serde functions for handling `Option<u64>` as [U64]
pub mod u64_hex_opt {
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

/// serde functions for handling primitive `u64` as [U64]
pub mod u64_hex_or_decimal {
    use crate::num::U64HexOrNumber;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u64` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        U64HexOrNumber::deserialize(deserializer).map(Into::into)
    }

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        U64HexOrNumber::from(*value).serialize(s)
    }
}

/// serde functions for handling primitive optional `u64` as [U64]
pub mod u64_hex_or_decimal_opt {
    use crate::num::U64HexOrNumber;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes an `u64` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<U64HexOrNumber>::deserialize(deserializer)? {
            Some(val) => Ok(Some(val.into())),
            None => Ok(None),
        }
    }

    /// Serializes u64 as hex string
    pub fn serialize<S: Serializer>(value: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(val) => U64HexOrNumber::from(*val).serialize(s),
            None => s.serialize_none(),
        }
    }
}

/// Deserializes the input into an `Option<U256>`, using [`from_int_or_hex`] to deserialize the
/// inner value.
pub fn from_int_or_hex_opt<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<NumberOrHexU256>::deserialize(deserializer)? {
        Some(val) => val.try_into_u256().map(Some),
        None => Ok(None),
    }
}

/// An enum that represents either a [serde_json::Number] integer, or a hex [U256].
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NumberOrHexU256 {
    /// An integer
    Int(serde_json::Number),
    /// A hex U256
    Hex(U256),
}

impl NumberOrHexU256 {
    /// Tries to convert this into a [U256]].
    pub fn try_into_u256<E: de::Error>(self) -> Result<U256, E> {
        match self {
            NumberOrHexU256::Int(num) => {
                U256::from_str(num.to_string().as_str()).map_err(E::custom)
            }
            NumberOrHexU256::Hex(val) => Ok(val),
        }
    }
}

/// Deserializes the input into a U256, accepting both 0x-prefixed hex and decimal strings with
/// arbitrary precision, defined by serde_json's [`Number`](serde_json::Number).
pub fn from_int_or_hex<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    NumberOrHexU256::deserialize(deserializer)?.try_into_u256()
}
/// serde functions for handling primitive `u128` as [U128](alloy_primitives::U128)
pub mod u128_hex_or_decimal {
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

/// serde functions for handling primitive optional `u128` as [U128](alloy_primitives::U128)
pub mod u128_hex_or_decimal_opt {
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

/// serde functions for handling `Vec<u128>` as [U128](alloy_primitives::U128)
pub mod u128_hex_or_decimal_vec {
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

/// serde functions for handling `Vec<Vec<u128>>` as [U128](alloy_primitives::U128)
pub mod u128_hex_or_decimal_vec_vec_opt {
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
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Value {
            #[serde(with = "u64_hex")]
            inner: u64,
        }

        let val = Value { inner: 1000 };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_hex_or_decimal() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Value {
            #[serde(with = "u128_hex_or_decimal")]
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
    fn test_u128_hex_or_decimal_opt() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Value {
            #[serde(with = "u128_hex_or_decimal_opt")]
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
    fn test_u128_hex_or_decimal_vec() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Value {
            #[serde(with = "u128_hex_or_decimal_vec")]
            inner: Vec<u128>,
        }

        let val = Value { inner: vec![1000, 2000] };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":[\"0x3e8\",\"0x7d0\"]}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_hex_or_decimal_vec_vec_opt() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Value {
            #[serde(with = "u128_hex_or_decimal_vec_vec_opt")]
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
