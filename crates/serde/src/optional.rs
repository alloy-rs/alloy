//! Serde functions for encoding optional values.
//!
//! This is defined as a "hex encoded unsigned integer", with a special case of 0 being `0x0`.
//!
//! A regex for this format is: `^0x([1-9a-f]+[0-9a-f]*|0)$`.
//!
//! This is only valid for human-readable [`serde`] implementations.
//! For non-human-readable implementations, the format is unspecified.
//! Currently, it uses a fixed-width big-endian byte-array.
use serde::{Deserialize, Deserializer};

/// For use with serde's `deserialize_with` on a sequence that must be
/// deserialized as a single but optional (i.e. possibly `null`) value.
pub fn or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let s: Option<T> = Deserialize::deserialize(deserializer)?;
    Ok(s.unwrap_or_default())
}
