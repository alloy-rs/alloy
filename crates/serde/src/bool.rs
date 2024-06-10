/// Serde serialization and deserialization for [`bool`] as `0x0` or `0x1`.
#[deprecated = "use `quantity::bool` instead"]
pub mod quantity_bool {
    use alloy_primitives::aliases::U1;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes a [`bool`] via a [U1] quantity.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        U1::deserialize(deserializer).map(|val| val.to())
    }

    /// Serializes a [`bool`] via a [U1] quantity.
    pub fn serialize<S: Serializer>(val: &bool, s: S) -> Result<S::Ok, S::Error> {
        if *val {
            "0x1".serialize(s)
        } else {
            "0x0".serialize(s)
        }
    }
}
