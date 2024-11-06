//! Support for capturing other fields.

use alloc::collections::BTreeMap;
use core::{
    fmt,
    ops::{Deref, DerefMut},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(any(test, feature = "arbitrary"))]
mod arbitrary_;

/// Generic type for capturing additional fields when deserializing structs.
///
/// For example, the [optimism `eth_getTransactionByHash` request][optimism] returns additional
/// fields that this type will capture instead.
///
/// Use `deserialize_as` or `deserialize_into` with a struct that captures the unknown fields, or
/// deserialize the individual fields manually with `get_deserialized`.
///
/// This type must be used with [`#[serde(flatten)]`][flatten].
///
/// [optimism]: https://docs.alchemy.com/alchemy/apis/optimism/eth-gettransactionbyhash
/// [flatten]: https://serde.rs/field-attrs.html#flatten
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OtherFields {
    inner: BTreeMap<String, serde_json::Value>,
}

impl OtherFields {
    /// Creates a new [`OtherFields`] instance.
    pub const fn new(inner: BTreeMap<String, serde_json::Value>) -> Self {
        Self { inner }
    }

    /// Deserialized this type into another container type.
    pub fn deserialize_as<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        let map = self.inner.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        serde_json::from_value(serde_json::Value::Object(map))
    }

    /// Deserialized this type into another container type.
    pub fn deserialize_into<T: DeserializeOwned>(self) -> serde_json::Result<T> {
        let map = self.inner.into_iter().collect();
        serde_json::from_value(serde_json::Value::Object(map))
    }

    /// Returns the deserialized value of the field, if it exists.
    /// Deserializes the value with the given closure
    pub fn get_with<F, V>(&self, key: impl AsRef<str>, with: F) -> Option<V>
    where
        F: FnOnce(serde_json::Value) -> V,
    {
        self.inner.get(key.as_ref()).cloned().map(with)
    }

    /// Returns the deserialized value of the field, if it exists
    pub fn get_deserialized<V: DeserializeOwned>(
        &self,
        key: impl AsRef<str>,
    ) -> Option<serde_json::Result<V>> {
        self.get_with(key, serde_json::from_value)
    }

    /// Removes the deserialized value of the field, if it exists
    ///
    /// **Note:** this will also remove the value if deserializing it resulted in an error
    pub fn remove_deserialized<V: DeserializeOwned>(
        &mut self,
        key: impl AsRef<str>,
    ) -> Option<serde_json::Result<V>> {
        self.inner.remove(key.as_ref()).map(serde_json::from_value)
    }

    /// Removes the deserialized value of the field, if it exists.
    /// Deserializes the value with the given closure
    ///
    /// **Note:** this will also remove the value if deserializing it resulted in an error
    pub fn remove_with<F, V>(&mut self, key: impl AsRef<str>, with: F) -> Option<V>
    where
        F: FnOnce(serde_json::Value) -> V,
    {
        self.inner.remove(key.as_ref()).map(with)
    }

    /// Removes the deserialized value of the field, if it exists and also returns the key
    ///
    /// **Note:** this will also remove the value if deserializing it resulted in an error
    pub fn remove_entry_deserialized<V: DeserializeOwned>(
        &mut self,
        key: impl AsRef<str>,
    ) -> Option<(String, serde_json::Result<V>)> {
        self.inner
            .remove_entry(key.as_ref())
            .map(|(key, value)| (key, serde_json::from_value(value)))
    }
}

impl fmt::Debug for OtherFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OtherFields ")?;
        self.inner.fmt(f)
    }
}

impl TryFrom<serde_json::Value> for OtherFields {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map(Self::new)
    }
}

impl<K> FromIterator<(K, serde_json::Value)> for OtherFields
where
    K: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, serde_json::Value)>>(iter: T) -> Self {
        Self { inner: iter.into_iter().map(|(key, value)| (key.into(), value)).collect() }
    }
}

impl Deref for OtherFields {
    type Target = BTreeMap<String, serde_json::Value>;

    #[inline]
    fn deref(&self) -> &BTreeMap<String, serde_json::Value> {
        self.as_ref()
    }
}

impl DerefMut for OtherFields {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsRef<BTreeMap<String, serde_json::Value>> for OtherFields {
    fn as_ref(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.inner
    }
}

impl IntoIterator for OtherFields {
    type Item = (String, serde_json::Value);
    type IntoIter = alloc::collections::btree_map::IntoIter<String, serde_json::Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a OtherFields {
    type Item = (&'a String, &'a serde_json::Value);
    type IntoIter = alloc::collections::btree_map::Iter<'a, String, serde_json::Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

/// An extension to a struct that allows to capture additional fields when deserializing.
///
/// See [`OtherFields`] for more information.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct WithOtherFields<T> {
    /// The inner struct.
    #[serde(flatten)]
    pub inner: T,
    /// All fields not present in the inner struct.
    #[serde(flatten)]
    pub other: OtherFields,
}

impl<T> WithOtherFields<T> {
    /// Creates a new [`WithOtherFields`] instance.
    pub fn new(inner: T) -> Self {
        Self { inner, other: Default::default() }
    }
}

impl<T> Deref for WithOtherFields<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for WithOtherFields<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'de, T> Deserialize<'de> for WithOtherFields<T>
where
    T: Deserialize<'de> + Serialize,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WithOtherFieldsHelper<T> {
            #[serde(flatten)]
            inner: T,
            #[serde(flatten)]
            other: OtherFields,
        }

        let mut helper = WithOtherFieldsHelper::deserialize(deserializer)?;
        // remove all fields present in the inner struct from the other fields, this is to avoid
        // duplicate fields in the catch all other fields because serde flatten does not exclude
        // already deserialized fields when deserializing the other fields.
        if let Value::Object(map) =
            serde_json::to_value(&helper.inner).map_err(serde::de::Error::custom)?
        {
            for key in map.keys() {
                helper.other.remove(key);
            }
        }

        Ok(Self { inner: helper.inner, other: helper.other })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use similar_asserts::assert_eq;

    #[test]
    fn other_fields_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = arbitrary::Unstructured::new(&bytes).arbitrary::<OtherFields>().unwrap();
    }

    #[test]
    fn test_correct_other() {
        #[derive(Serialize, Deserialize)]
        struct Inner {
            a: u64,
        }

        #[derive(Serialize, Deserialize)]
        struct InnerWrapper {
            #[serde(flatten)]
            inner: Inner,
        }

        let with_other: WithOtherFields<InnerWrapper> =
            serde_json::from_str("{\"a\": 1, \"b\": 2}").unwrap();
        assert_eq!(with_other.inner.inner.a, 1);
        assert_eq!(
            with_other.other,
            OtherFields::new(BTreeMap::from_iter([("b".to_string(), serde_json::json!(2))]))
        );
    }
}
