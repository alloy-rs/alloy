//! Some convenient serde helpers

use alloy_primitives::U256;
use serde::{Deserialize, Deserializer};

/// Supports parsing u64
///
/// See <https://github.com/gakonst/ethers-rs/issues/1507>
pub fn deserialize_stringified_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let num = U256::deserialize(deserializer)?;
    num.try_into().map_err(serde::de::Error::custom)
}

/// Supports parsing u64
///
/// See <https://github.com/gakonst/ethers-rs/issues/1507>
pub fn deserialize_stringified_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(num) = Option::<U256>::deserialize(deserializer)? {
        num.try_into().map(Some).map_err(serde::de::Error::custom)
    } else {
        Ok(None)
    }
}

/// Helper type to deserialize sequence of numbers
#[derive(Deserialize)]
#[serde(untagged)]
pub enum NumericSeq {
    Seq([U256; 1]),
    U256(U256),
    Num(u64),
}

/// Deserializes a number from hex or int
pub fn deserialize_number<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    U256::deserialize(deserializer)
}

/// Deserializes a number from hex or int, but optionally
pub fn deserialize_number_opt<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<U256>::deserialize(deserializer)
}

/// Deserializes single integer params: `1, [1], ["0x01"]`
pub fn deserialize_number_seq<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let num = match NumericSeq::deserialize(deserializer)? {
        NumericSeq::Seq(seq) => seq[0].into(),
        NumericSeq::U256(n) => n,
        NumericSeq::Num(n) => U256::from(n),
    };

    Ok(num)
}
