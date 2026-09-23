//! Support for capturing other fields.

use alloc::{collections::BTreeMap, format, string::String};
use core::{
    fmt,
    ops::{Deref, DerefMut},
};
use serde::{
    de::DeserializeOwned,
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Deserialize, Serialize, Serializer,
};
use serde_json::Value;

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

    /// Inserts a given value as serialized [`serde_json::Value`] into the map.
    pub fn insert_value(&mut self, key: String, value: impl Serialize) -> serde_json::Result<()> {
        self.inner.insert(key, serde_json::to_value(value)?);
        Ok(())
    }

    /// Inserts a given value as serialized [`serde_json::Value`] into the map and returns the
    /// updated instance.
    pub fn with_value(mut self, key: String, value: impl Serialize) -> serde_json::Result<Self> {
        self.insert_value(key, value)?;
        Ok(self)
    }

    /// Deserialized this type into another container type.
    pub fn deserialize_as<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::to_value(&self.inner).and_then(serde_json::from_value)
    }

    /// Deserialized this type into another container type.
    pub fn deserialize_into<T: DeserializeOwned>(self) -> serde_json::Result<T> {
        serde_json::from_value(serde_json::Value::Object(self.inner.into_iter().collect()))
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

    /// Returns the deserialized value of the field.
    ///
    /// Returns an error if the field is missing
    pub fn try_get_deserialized<V: DeserializeOwned>(
        &self,
        key: impl AsRef<str>,
    ) -> serde_json::Result<V> {
        let key = key.as_ref();
        self.get_deserialized(key)
            .ok_or_else(|| serde::de::Error::custom(format!("Missing field `{key}`")))?
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

impl<T, U> AsRef<U> for WithOtherFields<T>
where
    T: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        self.inner.as_ref()
    }
}

impl<T> WithOtherFields<T> {
    /// Creates a new [`WithOtherFields`] instance.
    pub fn new(inner: T) -> Self {
        Self { inner, other: Default::default() }
    }

    /// Consumes the type and returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Returns the wrapped value.
    pub const fn inner(&self) -> &T {
        &self.inner
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

        let mut helper: WithOtherFieldsHelper<T> =
            WithOtherFieldsHelper::deserialize(deserializer)?;
        // remove all fields present in the inner struct from the other fields, this is to avoid
        // duplicate fields in the catch all other fields because serde flatten does not exclude
        // already deserialized fields when deserializing the other fields.
        helper
            .inner
            .serialize(KnownFields { other: &mut helper.other, pending_key: None })
            .map_err(serde::de::Error::custom)?;

        Ok(Self { inner: helper.inner, other: helper.other })
    }
}

// Only the keys of the serialized inner object are needed here. In particular, serializing a
// receipt's logs into a Value would duplicate the entire (potentially large) log tree.
struct KnownFields<'a> {
    other: &'a mut OtherFields,
    pending_key: Option<String>,
}

// Use serde_json's map-key rules, including integer keys and errors for unsupported keys.
struct SingleKey<'a, T: ?Sized>(&'a T);

impl<T: Serialize + ?Sized> Serialize for SingleKey<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0, &())?;
        map.end()
    }
}

impl KnownFields<'_> {
    fn remove_key<T: Serialize + ?Sized>(&mut self, key: &T) -> serde_json::Result<()> {
        let Value::Object(map) = serde_json::to_value(SingleKey(key))? else { unreachable!() };
        if let Some(key) = map.keys().next() {
            self.other.remove(key);
        }
        Ok(())
    }
}

impl<'a> Serializer for KnownFields<'a> {
    type Ok = ();
    type Error = serde_json::Error;
    type SerializeSeq = IgnoreValues;
    type SerializeTuple = IgnoreValues;
    type SerializeTupleStruct = IgnoreValues;
    type SerializeTupleVariant = IgnoreValues;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = IgnoreValues;

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(IgnoreValues)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(IgnoreValues)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(IgnoreValues)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.other.remove(variant);
        Ok(IgnoreValues)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.other.remove(variant);
        Ok(IgnoreValues)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.other.remove(variant);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i128(self, _value: i128) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u128(self, _value: u128) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeMap for KnownFields<'_> {
    type Ok = ();
    type Error = serde_json::Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let Value::Object(map) = serde_json::to_value(SingleKey(key))? else { unreachable!() };
        self.pending_key = map.into_iter().next().map(|(key, _)| key);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        if let Some(key) = self.pending_key.take() {
            self.other.remove(&key);
        }
        Ok(())
    }

    fn serialize_entry<K: Serialize + ?Sized, V: Serialize + ?Sized>(
        &mut self,
        key: &K,
        _value: &V,
    ) -> Result<(), Self::Error> {
        self.remove_key(key)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStruct for KnownFields<'_> {
    type Ok = ();
    type Error = serde_json::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error> {
        self.other.remove(key);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct IgnoreValues;

impl SerializeSeq for IgnoreValues {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTuple for IgnoreValues {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleStruct for IgnoreValues {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleVariant for IgnoreValues {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStructVariant for IgnoreValues {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use rand::Rng;
    use serde_json::json;
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

    #[test]
    fn test_deserialize_only_serializes_inner_keys() {
        static LOG_SERIALIZATIONS: AtomicUsize = AtomicUsize::new(0);

        fn serialize_logs<S: Serializer>(logs: &Value, serializer: S) -> Result<S::Ok, S::Error> {
            LOG_SERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
            logs.serialize(serializer)
        }

        #[derive(Serialize, Deserialize)]
        struct Base {
            #[serde(rename = "knownField")]
            known_field: u64,
        }

        #[derive(Serialize, Deserialize)]
        struct Receipt {
            #[serde(flatten)]
            base: Base,
            #[serde(serialize_with = "serialize_logs")]
            logs: Value,
        }

        let receipt: WithOtherFields<Receipt> = serde_json::from_str(
            r#"{"knownField":1,"logs":[{"data":[1,2,3]}],"extra":1,"extra":2}"#,
        )
        .unwrap();

        assert_eq!(LOG_SERIALIZATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(receipt.inner.base.known_field, 1);
        assert_eq!(receipt.inner.logs, json!([{"data": [1, 2, 3]}]));
        assert_eq!(receipt.other.len(), 1);
        assert_eq!(receipt.other.get("extra"), Some(&json!(2)));

        let duplicate = serde_json::from_str::<WithOtherFields<Receipt>>(
            r#"{"knownField":1,"knownField":2,"logs":[]}"#,
        );
        assert!(duplicate.err().unwrap().to_string().contains("duplicate field `knownField`"));
    }

    #[test]
    fn test_with_other_fields_serialization() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Inner {
            a: u64,
            b: String,
        }

        let inner = Inner { a: 42, b: "Hello".to_string() };
        let mut other = BTreeMap::new();
        other.insert("extra".to_string(), json!(99));

        let with_other = WithOtherFields { inner, other: OtherFields::new(other.clone()) };
        let serialized = serde_json::to_string(&with_other).unwrap();

        let expected = r#"{"a":42,"b":"Hello","extra":99}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_remove_and_access_other_fields() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Inner {
            a: u64,
            b: String,
        }

        let json_data = r#"{"a":42,"b":"Hello","extra":99, "another": "test"}"#;
        let mut with_other: WithOtherFields<Inner> = serde_json::from_str(json_data).unwrap();

        assert_eq!(with_other.other.inner.get("extra"), Some(&json!(99)));
        assert_eq!(with_other.other.inner.get("another"), Some(&json!("test")));

        with_other.other.remove("extra");
        assert!(!with_other.other.inner.contains_key("extra"));
    }

    #[test]
    fn test_deserialize_as() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), json!(1));
        let other_fields = OtherFields::new(map);
        let deserialized: Result<BTreeMap<String, u64>, _> = other_fields.deserialize_as();
        assert_eq!(deserialized.unwrap().get("a"), Some(&1));
    }

    #[test]
    fn test_deserialize_into() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), json!(1));
        let other_fields = OtherFields::new(map);
        let deserialized: Result<BTreeMap<String, u64>, _> = other_fields.deserialize_into();
        assert_eq!(deserialized.unwrap().get("a"), Some(&1));
    }

    #[test]
    fn test_get_with() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), json!(42));
        let other_fields = OtherFields::new(map);
        let value: Option<u64> = other_fields.get_with("key", |v| v.as_u64().unwrap());
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_get_deserialized() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), json!(42));
        let other_fields = OtherFields::new(map);
        let value: Option<serde_json::Result<u64>> = other_fields.get_deserialized("key");
        assert_eq!(value.unwrap().unwrap(), 42);
    }

    #[test]
    fn test_remove_deserialized() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), json!(42));
        let mut other_fields = OtherFields::new(map);
        let value: Option<serde_json::Result<u64>> = other_fields.remove_deserialized("key");
        assert_eq!(value.unwrap().unwrap(), 42);
        assert!(!other_fields.inner.contains_key("key"));
    }

    #[test]
    fn test_remove_with() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), json!(42));
        let mut other_fields = OtherFields::new(map);
        let value: Option<u64> = other_fields.remove_with("key", |v| v.as_u64().unwrap());
        assert_eq!(value, Some(42));
        assert!(!other_fields.inner.contains_key("key"));
    }

    #[test]
    fn test_remove_entry_deserialized() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), json!(42));
        let mut other_fields = OtherFields::new(map);
        let entry: Option<(String, serde_json::Result<u64>)> =
            other_fields.remove_entry_deserialized("key");
        assert!(entry.is_some());
        let (key, value) = entry.unwrap();
        assert_eq!(key, "key");
        assert_eq!(value.unwrap(), 42);
        assert!(!other_fields.inner.contains_key("key"));
    }

    #[test]
    fn test_try_from_value() {
        let json_value = json!({ "key": "value" });
        let other_fields = OtherFields::try_from(json_value).unwrap();
        assert_eq!(other_fields.inner.get("key").unwrap(), &json!("value"));
    }

    #[test]
    fn test_into_iter() {
        let mut map = BTreeMap::new();
        map.insert("key1".to_string(), json!("value1"));
        map.insert("key2".to_string(), json!("value2"));
        let other_fields = OtherFields::new(map.clone());

        let iterated_map: BTreeMap<_, _> = other_fields.into_iter().collect();
        assert_eq!(iterated_map, map);
    }
}
