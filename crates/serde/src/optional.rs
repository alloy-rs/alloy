//! Serde functions for encoding optional values.
use serde::{Deserialize, Deserializer};

/// For use with serde's `deserialize_with` on a sequence that must be
/// deserialized as a single but optional (i.e. possibly `null`) value.
pub fn null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let s: Option<T> = Deserialize::deserialize(deserializer).ok();
    Ok(s.unwrap_or_default())
}
