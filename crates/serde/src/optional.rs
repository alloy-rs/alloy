//! Serde functions for encoding optional values.

use serde::{Deserialize, Deserializer};

/// For use with serde's `deserialize_with` on a sequence that must be
/// deserialized as a single but optional (i.e. possibly `null`) value.
pub fn null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// For use with serde's `deserialize_with` on a field that must be missing.
pub fn reject_if_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<T>::deserialize(deserializer)?;

    if value.is_some() {
        return Err(serde::de::Error::custom("unexpected value"));
    }

    Ok(value)
}
