//! Support for capturing other fields
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Map;
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

/// A type that is supposed to capture additional fields that are not native to ethereum but included in ethereum adjacent networks, for example fields the [optimism `eth_getTransactionByHash` request](https://docs.alchemy.com/alchemy/apis/optimism/eth-gettransactionbyhash) returns additional fields that this type will capture
///
/// This type is supposed to be used with [`#[serde(flatten)`](https://serde.rs/field-attrs.html#flatten)
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct OtherFields {
    /// Contains all unknown fields
    inner: BTreeMap<String, serde_json::Value>,
}

// === impl OtherFields ===

impl OtherFields {
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
        self.inner.get(key.as_ref()).cloned().map(serde_json::from_value)
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

    /// Deserialized this type into another container type
    pub fn deserialize_into<T: DeserializeOwned>(self) -> serde_json::Result<T> {
        let mut map = Map::with_capacity(self.inner.len());
        map.extend(self);
        serde_json::from_value(serde_json::Value::Object(map))
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
    type IntoIter = std::collections::btree_map::IntoIter<String, serde_json::Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a OtherFields {
    type Item = (&'a String, &'a serde_json::Value);
    type IntoIter = std::collections::btree_map::Iter<'a, String, serde_json::Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}
