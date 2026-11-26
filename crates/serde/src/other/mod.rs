//! Support for capturing other fields.

use alloc::{collections::BTreeMap, format, string::String};
use core::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::Value;

#[cfg(any(test, feature = "arbitrary"))]
mod arbitrary_;

mod buffered;
mod keys;

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
        struct ObjectVisitor<T>(PhantomData<fn() -> T>);

        impl<'de, T: Deserialize<'de> + Serialize> Visitor<'de> for ObjectVisitor<T> {
            type Value = WithOtherFields<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("struct WithOtherFieldsHelper")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                // Buffer the object once. The inner value reads the entries by reference, and
                // only the entries it neither took nor serializes become other fields.
                let mut entries = buffered::Entries::read(map)?;
                if entries.has_non_json() {
                    // A byte string cannot become another field, wherever it is, and neither can
                    // a map with a key that is no string. Whether that is an error depends on
                    // what the inner value leaves in place, which serde's own flattening settles.
                    return deserialize_flattened(buffered::Ref::new(&entries.into_object()));
                }
                let inner = T::deserialize(buffered::Flat::new(&mut entries));
                if entries.asked_for_option() {
                    // serde decides through a private hook whether a flattened `Option` is
                    // `None`, so that case goes through its own flattening.
                    return deserialize_flattened(buffered::Ref::new(&entries.into_object()));
                }
                let inner = inner?;
                entries.remove_keys_of(&inner).map_err(serde::de::Error::custom)?;

                Ok(WithOtherFields { inner, other: OtherFields::new(entries.into_other()?) })
            }
        }

        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

/// Deserializes through `#[serde(flatten)]`, which buffers the object and copies every field the
/// inner value leaves in place into the other fields before removing them again.
fn deserialize_flattened<'de, T, D>(deserializer: D) -> Result<WithOtherFields<T>, D::Error>
where
    T: Deserialize<'de> + Serialize,
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
    if keys::remove_keys(&helper.inner, &mut helper.other.inner).is_err() {
        // Not every key is a string: let `serde_json` stringify them.
        if let Value::Object(map) =
            serde_json::to_value(&helper.inner).map_err(serde::de::Error::custom)?
        {
            for key in map.keys() {
                helper.other.remove(key);
            }
        }
    }

    Ok(WithOtherFields { inner: helper.inner, other: helper.other })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{
        string::{String, ToString},
        vec::Vec,
    };
    use alloy_primitives::B256;
    use arbitrary::{Arbitrary, Unstructured};
    use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
    use serde::de::{
        value::{self, MapDeserializer},
        IntoDeserializer,
    };
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
    fn test_other_excludes_serialized_keys() {
        #[derive(Serialize, Deserialize)]
        struct Inner {
            a: u64,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            skipped: Option<u64>,
            #[serde(flatten)]
            map: BTreeMap<String, Value>,
        }

        let with_other: WithOtherFields<Inner> =
            serde_json::from_str(r#"{"a": 1, "skipped": null, "x": 2}"#).unwrap();
        assert_eq!(with_other.inner.map, BTreeMap::from([("x".to_string(), json!(2))]));
        assert_eq!(
            with_other.other,
            OtherFields::new(BTreeMap::from([("skipped".to_string(), json!(null))]))
        );

        #[derive(Deserialize)]
        struct IntKeys(BTreeMap<String, Value>);

        impl Serialize for IntKeys {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_map(self.0.iter().map(|(k, v)| (k.parse::<u64>().unwrap(), v)))
            }
        }

        let with_other: WithOtherFields<IntKeys> =
            serde_json::from_str(r#"{"1": 2, "2": 3}"#).unwrap();
        assert_eq!(with_other.inner.0.len(), 2);
        assert!(with_other.other.is_empty());
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

    /// Decodes `json` through the buffer and through `#[serde(flatten)]`, as text, from a reader
    /// and from a `Value`, expects the same result each time and returns whether it decoded.
    fn matches_flattened<T>(json: &str) -> bool
    where
        T: DeserializeOwned + Serialize + PartialEq + fmt::Debug,
    {
        let buffered =
            serde_json::from_str::<WithOtherFields<T>>(json).map_err(|err| err.to_string());
        let mut deserializer = serde_json::Deserializer::from_str(json);
        let flattened =
            deserialize_flattened::<T, _>(&mut deserializer).map_err(|err| err.to_string());
        assert_eq!(buffered, flattened, "{json}");

        // A reader hands out owned strings.
        let mut deserializer = serde_json::Deserializer::from_reader(json.as_bytes());
        assert_eq!(
            serde_json::from_reader::<_, WithOtherFields<T>>(json.as_bytes())
                .map_err(|err| err.to_string()),
            deserialize_flattened::<T, _>(&mut deserializer).map_err(|err| err.to_string()),
            "{json}"
        );

        // A `Value` keeps the last of a repeated key.
        if let Ok(value) = serde_json::from_str::<Value>(json) {
            assert_eq!(
                serde_json::from_value::<WithOtherFields<T>>(value.clone())
                    .map_err(|err| err.to_string()),
                deserialize_flattened::<T, _>(value).map_err(|err| err.to_string()),
                "{json}"
            );
        }

        buffered.is_ok()
    }

    #[test]
    fn deserialize_matches_flattened() {
        /// Takes the entries it names. `b` is read but not written back when it is `None`, and
        /// serde reads the units of taken entries from an empty object or array too.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Named {
            a: Option<u64>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            b: Option<String>,
            #[serde(default)]
            c: Vec<u64>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            x: Option<Unit>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            y: Option<()>,
            #[serde(default)]
            z: Vec<Unit>,
        }

        /// Sees all entries and leaves them in place.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Flattening {
            #[serde(flatten)]
            named: Named,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Newtype(Named);

        /// Takes the entry named like one of its variants.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        enum Tagged {
            A(u64),
            B { x: u64 },
            C,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(untagged)]
        enum Untagged {
            Named(Named),
            Map(BTreeMap<String, Value>),
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Unit;

        /// A map key that is read as a newtype.
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        struct Id(String);

        /// A map key that is read as an enum.
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        enum Kind {
            A,
            B,
        }

        /// Takes maps whose keys are no plain strings.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Keyed {
            #[serde(default)]
            ids: BTreeMap<Id, u64>,
            #[serde(default)]
            kinds: BTreeMap<Kind, u64>,
            #[serde(default)]
            options: BTreeMap<Option<String>, u64>,
        }

        /// Leaves them in place.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct FlatteningKeyed {
            #[serde(flatten)]
            keyed: Keyed,
        }

        const LIMIT: usize = buffered::LINEAR_SEARCH_LIMIT;
        const KEYS: &[&str] = &["a", "b", "c", "x", "y", "z"];
        const VALUES: &[&str] = &[
            "null",
            "0",
            "7",
            "-1",
            "1.5",
            "true",
            r#""s""#,
            "[]",
            "[1,2]",
            r#"[1,"s"]"#,
            "{}",
            r#"{"x":1}"#,
            r#"{"x":1,"x":2}"#,
            "[{}]",
            "[[],null]",
        ];
        const KEYED_KEYS: [&str; 3] = ["ids", "kinds", "options"];
        const KEYED_VALUES: &[&str] =
            &["{}", r#"{"a":1}"#, r#"{"a":1,"b":2}"#, r#"{"x":1}"#, r#"{"a":{}}"#, "null"];

        let mut rng = StdRng::seed_from_u64(0);
        let mut bytes = [0u8; 256];
        let mut decoded = 0;
        let mut decoded_keyed = [0; 3];
        let mut decoded_many = [0; 2];
        for _ in 0..500 {
            // Fields no inner type knows, with keys that need escaping.
            rng.fill(bytes.as_mut_slice());
            let unknown = OtherFields::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
            let mut entries: Vec<(String, String)> = unknown
                .iter()
                .map(|(key, value)| (serde_json::to_string(key).unwrap(), value.to_string()))
                .collect();
            // Fields the inner types know, valid or not, possibly repeated.
            for _ in 0..rng.gen_range(0..6) {
                let key = KEYS[rng.gen_range(0..KEYS.len())];
                let value = VALUES[rng.gen_range(0..VALUES.len())];
                entries.push((format!("{key:?}"), value.to_string()));
            }
            // Maps whose keys are read as a newtype, as an enum or as an option.
            for _ in 0..rng.gen_range(0..3) {
                let key = KEYED_KEYS[rng.gen_range(0..KEYED_KEYS.len())];
                let value = KEYED_VALUES[rng.gen_range(0..KEYED_VALUES.len())];
                entries.push((format!("{key:?}"), value.to_string()));
            }
            // More entries than are searched one by one to remove a key, with repeated keys.
            let many = rng.gen_ratio(1, 3);
            if many {
                for _ in 0..rng.gen_range(LIMIT..3 * LIMIT) {
                    let key = format!("k{}", rng.gen_range(0..2 * LIMIT));
                    entries.push((format!("{key:?}"), rng.gen_range(0..9).to_string()));
                }
            }
            entries.shuffle(&mut rng);
            let fields: Vec<String> =
                entries.iter().map(|(key, value)| format!("{key}:{value}")).collect();
            let json = format!("{{{}}}", fields.join(","));

            let named = matches_flattened::<Named>(&json);
            decoded += usize::from(named);
            decoded_many[0] += usize::from(many && named);
            let keyed = matches_flattened::<Keyed>(&json);
            for (decoded, key) in decoded_keyed.iter_mut().zip(KEYED_KEYS) {
                *decoded += usize::from(keyed && json.contains(&format!(r#""{key}":{{""#)));
            }
            matches_flattened::<FlatteningKeyed>(&json);
            matches_flattened::<BTreeMap<Id, Value>>(&json);
            matches_flattened::<BTreeMap<Kind, Value>>(&json);
            matches_flattened::<Flattening>(&json);
            matches_flattened::<Newtype>(&json);
            matches_flattened::<Tagged>(&json);
            matches_flattened::<Untagged>(&json);
            matches_flattened::<Unit>(&json);
            matches_flattened::<Option<Named>>(&json);
            let map = matches_flattened::<BTreeMap<String, Value>>(&json);
            decoded_many[1] += usize::from(many && map);
            matches_flattened::<WithOtherFields<Named>>(&json);
            // Not something that can be flattened.
            matches_flattened::<u64>(&json);
            matches_flattened::<Vec<u64>>(&json);
        }
        // Enough objects decode for the comparison to mean something.
        assert!(decoded > 50, "{decoded}");
        assert!(decoded_keyed.iter().all(|decoded| *decoded > 20), "{decoded_keyed:?}");
        assert!(decoded_many.iter().all(|decoded| *decoded > 20), "{decoded_many:?}");

        // Around the limit, with a repeated key. The inner value keeps one key, all or none.
        for entries in LIMIT - 1..=LIMIT + 2 {
            let fields: Vec<String> =
                (0..entries).map(|i| format!(r#""{}":{i}"#, ["a", "k"][i.min(1)])).collect();
            let json = format!("{{{}}}", fields.join(","));
            assert!(matches_flattened::<Named>(&json));
            assert!(matches_flattened::<BTreeMap<String, Value>>(&json));
            assert!(matches_flattened::<Unit>(&json));
        }

        for json in ["{}", "[]", "null", "1", r#""a""#] {
            matches_flattened::<Named>(json);
            matches_flattened::<Option<Named>>(json);
        }
    }

    /// Formats other than JSON have byte strings and keys that are no strings, which the buffer
    /// has to keep.
    #[test]
    fn deserialize_non_json_matches_flattened() {
        #[derive(Clone, Copy)]
        enum Input {
            Str(&'static str),
            Bytes(&'static [u8]),
            Number(u64),
            Map(&'static [(Self, Self)]),
        }

        impl<'de> serde::Deserializer<'de> for Input {
            type Error = value::Error;

            fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
                match self {
                    Self::Str(string) => visitor.visit_borrowed_str(string),
                    Self::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
                    Self::Number(number) => visitor.visit_u64(number),
                    Self::Map(entries) => {
                        visitor.visit_map(MapDeserializer::new(entries.iter().copied()))
                    }
                }
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
                byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct
                enum identifier ignored_any
            }
        }

        impl IntoDeserializer<'_, value::Error> for Input {
            type Deserializer = Self;

            fn into_deserializer(self) -> Self {
                self
            }
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Named {
            hash: B256,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Flattening {
            #[serde(flatten)]
            named: Named,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Limits {
            limits: BTreeMap<u64, u64>,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct FlatteningLimits {
            #[serde(flatten)]
            limits: Limits,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Names {
            names: BTreeMap<String, u64>,
        }

        /// Decodes `input` with the buffer, which has to agree with serde's own flattening.
        fn decode<T>(input: Input) -> Result<WithOtherFields<T>, String>
        where
            T: DeserializeOwned + Serialize + PartialEq + fmt::Debug,
        {
            let buffered = WithOtherFields::<T>::deserialize(input).map_err(|err| err.to_string());
            assert_eq!(buffered, deserialize_flattened(input).map_err(|err| err.to_string()));
            buffered
        }

        const HASH: Input = Input::Bytes(&[0x11; 32]);
        let height = || OtherFields::new(BTreeMap::from([("height".to_string(), json!(7))]));

        // A struct takes its byte string out of the entries.
        let object =
            Input::Map(&[(Input::Str("hash"), HASH), (Input::Str("height"), Input::Number(7))]);
        let named = decode::<Named>(object).unwrap();
        assert_eq!(named.inner.hash, B256::repeat_byte(0x11));
        assert_eq!(named.other, height());

        // A flattened field leaves it in place, where it is no valid other field.
        assert_eq!(
            decode::<Flattening>(object).unwrap_err(),
            "invalid type: byte array, expected any valid JSON value"
        );

        // A key of the object may come as bytes.
        let object =
            Input::Map(&[(Input::Bytes(b"hash"), HASH), (Input::Str("height"), Input::Number(7))]);
        let named = decode::<Named>(object).unwrap();
        assert_eq!(named.inner.hash, B256::repeat_byte(0x11));
        assert_eq!(named.other, height());

        // A key that is no string is no other field.
        let object = Input::Map(&[
            (Input::Str("limits"), Input::Map(&[])),
            (Input::Number(1), Input::Number(7)),
        ]);
        assert_eq!(
            decode::<Limits>(object).unwrap_err(),
            "invalid type: integer `1`, expected a string"
        );

        // A struct takes its map out of the entries, whatever the keys are.
        let object = Input::Map(&[
            (Input::Str("limits"), Input::Map(&[(Input::Number(1), Input::Number(7))])),
            (Input::Str("height"), Input::Number(7)),
        ]);
        let limits = decode::<Limits>(object).unwrap();
        assert_eq!(limits.inner.limits, BTreeMap::from([(1, 7)]));
        assert_eq!(limits.other, height());

        // A flattened field leaves it in place, where an integer is no key of another field.
        assert_eq!(
            decode::<FlatteningLimits>(object).unwrap_err(),
            "invalid type: integer `1`, expected a string key"
        );

        // The key of a nested map may come as bytes too.
        let object = Input::Map(&[
            (Input::Str("names"), Input::Map(&[(Input::Bytes(b"a"), Input::Number(7))])),
            (Input::Str("height"), Input::Number(7)),
        ]);
        let names = decode::<Names>(object).unwrap();
        assert_eq!(names.inner.names, BTreeMap::from([("a".to_string(), 7)]));
        assert_eq!(names.other, height());
    }
}
