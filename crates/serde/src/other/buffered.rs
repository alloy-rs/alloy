//! A borrowed buffer for an object that is decoded twice: once into the inner value of a
//! [`WithOtherFields`](super::WithOtherFields), and once into the fields that value did not use.
//!
//! This mirrors what serde's `#[serde(flatten)]` does with its private `Content` buffer, which
//! does not let the second reader skip the entries the first one used. [`Flat`] and [`Ref`] answer
//! every hint the way serde's `FlatMapDeserializer` and `ContentRefDeserializer` do.

use super::keys::{self, KeySet};
use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::{cell::Cell, fmt, marker::PhantomData, mem::size_of};
use serde::{
    de::{
        self,
        value::{MapDeserializer, SeqDeserializer},
        DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, Unexpected,
        VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer, Serialize,
};
use serde_json::Value;

/// Objects with more entries than this get a sorted index to find the entries with a key.
/// Searching every entry for every key is quadratic if the inner value keeps all of them.
pub(super) const LINEAR_SEARCH_LIMIT: usize = 32;

/// A buffered value, as the input handed it out. Strings borrow from the input where the input
/// allows it.
#[derive(Debug)]
pub(super) enum Buffered<'de> {
    /// The unit value, which is what JSON's `null` is read as.
    Null,
    None,
    Some(Box<Self>),
    Newtype(Box<Self>),
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Char(char),
    Str(Cow<'de, str>),
    Bytes(Cow<'de, [u8]>),
    Seq(Vec<Self>),
    Map(Vec<(Cow<'de, str>, Self)>),
    /// A map with a key that is no string. Only serde's own flattening reads one, as a whole.
    Pairs(Vec<(Self, Self)>),
}

impl Buffered<'_> {
    fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Self::Null => Unexpected::Unit,
            Self::None | Self::Some(_) => Unexpected::Option,
            Self::Newtype(_) => Unexpected::NewtypeStruct,
            Self::Bool(value) => Unexpected::Bool(*value),
            Self::U8(value) => Unexpected::Unsigned((*value).into()),
            Self::U16(value) => Unexpected::Unsigned((*value).into()),
            Self::U32(value) => Unexpected::Unsigned((*value).into()),
            Self::U64(value) => Unexpected::Unsigned(*value),
            Self::I8(value) => Unexpected::Signed((*value).into()),
            Self::I16(value) => Unexpected::Signed((*value).into()),
            Self::I32(value) => Unexpected::Signed((*value).into()),
            Self::I64(value) => Unexpected::Signed(*value),
            Self::F32(value) => Unexpected::Float((*value).into()),
            Self::F64(value) => Unexpected::Float(*value),
            Self::Char(value) => Unexpected::Char(*value),
            Self::Str(value) => Unexpected::Str(value),
            Self::Bytes(value) => Unexpected::Bytes(value),
            Self::Seq(_) => Unexpected::Seq,
            Self::Map(_) | Self::Pairs(_) => Unexpected::Map,
        }
    }
}

/// Buffers a value and notes whether it holds what cannot be JSON: a byte string, a newtype, or a
/// map with a key that is no string.
#[derive(Clone, Copy)]
struct BufferedSeed<'a>(&'a Cell<bool>);

impl<'de> BufferedSeed<'_> {
    /// Buffers a map from `key` on, its first key that is no string.
    fn pairs<A: MapAccess<'de>>(
        self,
        entries: impl Iterator<Item = (Cow<'de, str>, Buffered<'de>)>,
        key: Buffered<'de>,
        map: &mut A,
    ) -> Result<Buffered<'de>, A::Error> {
        self.0.set(true);
        let mut pairs: Vec<_> = entries.map(|(key, value)| (Buffered::Str(key), value)).collect();
        pairs.push((key, map.next_value_seed(self)?));
        while let Some(key) = map.next_key_seed(self)? {
            pairs.push((key, map.next_value_seed(self)?));
        }
        Ok(Buffered::Pairs(pairs))
    }
}

impl<'de> DeserializeSeed<'de> for BufferedSeed<'_> {
    type Value = Buffered<'de>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for BufferedSeed<'_> {
    type Value = Buffered<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Buffered::Bool(value))
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> {
        Ok(Buffered::U8(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> {
        Ok(Buffered::U16(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> {
        Ok(Buffered::U32(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Buffered::U64(value))
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E> {
        Ok(Buffered::I8(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E> {
        Ok(Buffered::I16(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E> {
        Ok(Buffered::I32(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Buffered::I64(value))
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
        Ok(Buffered::F32(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Buffered::F64(value))
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E> {
        Ok(Buffered::Char(value))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Buffered::Str(Cow::Borrowed(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Buffered::Str(Cow::Owned(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Buffered::Str(Cow::Owned(value)))
    }

    fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        self.0.set(true);
        Ok(Buffered::Bytes(Cow::Borrowed(value)))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.0.set(true);
        Ok(Buffered::Bytes(Cow::Owned(value.to_vec())))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        self.0.set(true);
        Ok(Buffered::Bytes(Cow::Owned(value)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Buffered::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Buffered::None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        Ok(Buffered::Some(Box::new(deserializer.deserialize_any(self)?)))
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        self.0.set(true);
        Ok(Buffered::Newtype(Box::new(deserializer.deserialize_any(self)?)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut items = with_capacity_hint(seq.size_hint());
        while let Some(item) = seq.next_element_seed(self)? {
            items.push(item);
        }
        Ok(Buffered::Seq(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut entries = with_capacity_hint(map.size_hint());
        while let Some(key) = map.next_key_seed(KeySeed(self))? {
            match key {
                Key::Str(key) => entries.push((key, map.next_value_seed(self)?)),
                Key::Other(key) => return self.pairs(entries.into_iter(), *key, &mut map),
            }
        }
        Ok(Buffered::Map(entries))
    }
}

/// A vector with room for the number of elements the input states, up to 1 MiB of them as in
/// serde's own buffer: the input may state any length.
fn with_capacity_hint<T>(hint: Option<usize>) -> Vec<T> {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    Vec::with_capacity(hint.unwrap_or(0).min(MAX_PREALLOC_BYTES / size_of::<T>().max(1)))
}

/// A map key as it is read. JSON only has strings.
enum Key<'de> {
    Str(Cow<'de, str>),
    Other(Box<Buffered<'de>>),
}

impl<'de> Key<'de> {
    fn other(key: Buffered<'de>) -> Self {
        Self::Other(Box::new(key))
    }
}

/// Reads a map key, borrowing a string where the input allows it.
struct KeySeed<'a>(BufferedSeed<'a>);

impl<'de> DeserializeSeed<'de> for KeySeed<'_> {
    type Value = Key<'de>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

macro_rules! visit_other_key {
    ($($method:ident($ty:ty))*) => {$(
        fn $method<E: de::Error>(self, key: $ty) -> Result<Self::Value, E> {
            self.0.$method(key).map(Key::other)
        }
    )*};
}

impl<'de> Visitor<'de> for KeySeed<'_> {
    type Value = Key<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a map key")
    }

    fn visit_borrowed_str<E>(self, key: &'de str) -> Result<Self::Value, E> {
        Ok(Key::Str(Cow::Borrowed(key)))
    }

    fn visit_str<E>(self, key: &str) -> Result<Self::Value, E> {
        Ok(Key::Str(Cow::Owned(key.to_string())))
    }

    fn visit_string<E>(self, key: String) -> Result<Self::Value, E> {
        Ok(Key::Str(Cow::Owned(key)))
    }

    visit_other_key! {
        visit_bool(bool) visit_u8(u8) visit_u16(u16) visit_u32(u32) visit_u64(u64) visit_i8(i8)
        visit_i16(i16) visit_i32(i32) visit_i64(i64) visit_f32(f32) visit_f64(f64)
        visit_char(char) visit_borrowed_bytes(&'de [u8]) visit_bytes(&[u8])
        visit_byte_buf(Vec<u8>)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_unit().map(Key::other)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_none().map(Key::other)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.0.visit_some(deserializer).map(Key::other)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        self.0.visit_newtype_struct(deserializer).map(Key::other)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.0.visit_seq(seq).map(Key::other)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        self.0.visit_map(map).map(Key::other)
    }
}

/// The entries of the buffered object. An entry is `None` once a reader took it.
#[derive(Debug)]
pub(super) struct Entries<'de> {
    entries: Vec<Option<(Cow<'de, str>, Buffered<'de>)>>,
    /// The whole object instead, if one of its keys is no string.
    pairs: Option<Buffered<'de>>,
    has_non_json: bool,
    asked_for_option: bool,
}

impl<'de> Entries<'de> {
    /// Buffers the entries of `map`.
    pub(super) fn read<A: MapAccess<'de>>(mut map: A) -> Result<Self, A::Error> {
        let has_non_json = Cell::new(false);
        let seed = BufferedSeed(&has_non_json);
        let mut entries = with_capacity_hint(map.size_hint());
        let mut pairs = None;
        while let Some(key) = map.next_key_seed(KeySeed(seed))? {
            match key {
                Key::Str(key) => entries.push(Some((key, map.next_value_seed(seed)?))),
                Key::Other(key) => {
                    pairs = Some(seed.pairs(entries.drain(..).flatten(), *key, &mut map)?);
                    break;
                }
            }
        }
        Ok(Self { entries, pairs, has_non_json: has_non_json.get(), asked_for_option: false })
    }

    /// Whether the object holds a byte string, a newtype or a key that is no string. None of them
    /// can be JSON, and other fields are JSON.
    pub(super) const fn has_non_json(&self) -> bool {
        self.has_non_json
    }

    /// Whether the inner value asked [`Flat`] for an `Option`, which it cannot answer.
    pub(super) const fn asked_for_option(&self) -> bool {
        self.asked_for_option
    }

    /// Returns the entries that are left as a map, for serde's own flattening to read.
    pub(super) fn into_object(self) -> Buffered<'de> {
        match self.pairs {
            Some(pairs) => pairs,
            None => Buffered::Map(self.entries.into_iter().flatten().collect()),
        }
    }

    /// Removes the entries with the top-level keys that `value` serializes to. They are fields of
    /// the inner value, not other fields.
    pub(super) fn remove_keys_of<T: Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        if self.entries.len() <= LINEAR_SEARCH_LIMIT {
            return remove_serialized_keys(value, self);
        }
        let mut keys = SortedKeys::new(&self.entries);
        let result = remove_serialized_keys(value, &mut keys);
        for position in keys.removed {
            self.entries[position] = None;
        }
        result
    }

    /// Returns the entries that are left, as the other fields of the object.
    pub(super) fn into_other<E: de::Error>(self) -> Result<BTreeMap<String, Value>, E> {
        let mut other = BTreeMap::new();
        for (key, value) in self.entries.into_iter().flatten() {
            other.insert(key.into_owned(), Value::deserialize(Ref::<E>::new(&value))?);
        }
        Ok(other)
    }
}

impl KeySet for Entries<'_> {
    fn remove_key(&mut self, key: &str) {
        for entry in &mut self.entries {
            if entry.as_ref().is_some_and(|(name, _)| name == key) {
                *entry = None;
            }
        }
    }
}

/// The keys of many entries, sorted to find the entries with a key without reading all of them.
struct SortedKeys<'a> {
    /// The key and the position of every entry that is left.
    sorted: Vec<(&'a str, usize)>,
    /// The positions of the entries to remove.
    removed: Vec<usize>,
}

impl<'a> SortedKeys<'a> {
    fn new(entries: &'a [Option<(Cow<'_, str>, Buffered<'_>)>]) -> Self {
        let mut sorted: Vec<_> = entries
            .iter()
            .enumerate()
            .filter_map(|(position, entry)| Some((&*entry.as_ref()?.0, position)))
            .collect();
        sorted.sort_unstable();
        Self { sorted, removed: Vec::new() }
    }
}

impl KeySet for SortedKeys<'_> {
    fn remove_key(&mut self, key: &str) {
        // The entries of a repeated key are next to each other.
        let start = self.sorted.partition_point(|(name, _)| *name < key);
        let named = self.sorted[start..].iter().take_while(|(name, _)| *name == key);
        self.removed.extend(named.map(|(_, position)| position));
    }
}

/// Removes the top-level keys that `value` serializes to from `keys`.
fn remove_serialized_keys<T: Serialize, M: KeySet>(
    value: &T,
    keys: &mut M,
) -> Result<(), serde_json::Error> {
    if keys::remove_keys(value, keys).is_err() {
        // Not every key is a string: let `serde_json` stringify them.
        if let Value::Object(object) = serde_json::to_value(value)? {
            for key in object.keys() {
                keys.remove_key(key);
            }
        }
    }
    Ok(())
}

/// Reads a buffered key. As in serde's own buffer, a string is also read as an option, as a
/// newtype and as the name of a variant.
struct KeyRef<'a, 'de, E>(&'a Cow<'de, str>, PhantomData<E>);

impl<'de, E: de::Error> Deserializer<'de> for KeyRef<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.0 {
            Cow::Borrowed(key) => visitor.visit_borrowed_str(key),
            Cow::Owned(key) => visitor.visit_str(key),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_enum(EnumRef {
            variant: self.0,
            value: None,
            taken: false,
            marker: PhantomData,
        })
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

impl<'de, E: de::Error> IntoDeserializer<'de, E> for KeyRef<'_, 'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Reads a buffered value by reference, with the same hints as serde's own buffer.
pub(super) struct Ref<'a, 'de, E> {
    value: &'a Buffered<'de>,
    /// Whether the value was taken out of the entries. serde reads the units of those with rules
    /// of their own.
    taken: bool,
    marker: PhantomData<E>,
}

impl<'a, 'de, E> Ref<'a, 'de, E> {
    pub(super) const fn new(value: &'a Buffered<'de>) -> Self {
        Self { value, taken: false, marker: PhantomData }
    }

    const fn taken(value: &'a Buffered<'de>) -> Self {
        Self { value, taken: true, marker: PhantomData }
    }

    const fn with_taken(value: &'a Buffered<'de>, taken: bool) -> Self {
        Self { value, taken, marker: PhantomData }
    }
}

impl<'de, E: de::Error> Ref<'_, 'de, E> {
    fn invalid_type<V: Visitor<'de>>(&self, visitor: &V) -> E {
        E::invalid_type(self.value.unexpected(), visitor)
    }

    fn visit_seq<V: Visitor<'de>>(
        items: &[Buffered<'de>],
        taken: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut seq =
            SeqDeserializer::new(items.iter().map(|item| Ref::<E>::with_taken(item, taken)));
        let value = visitor.visit_seq(&mut seq)?;
        seq.end()?;
        Ok(value)
    }

    fn visit_map<V: Visitor<'de>>(
        entries: &[(Cow<'de, str>, Buffered<'de>)],
        taken: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut map =
            MapDeserializer::new(entries.iter().map(|(key, value)| {
                (KeyRef(key, PhantomData), Ref::<E>::with_taken(value, taken))
            }));
        let value = visitor.visit_map(&mut map)?;
        map.end()?;
        Ok(value)
    }

    fn visit_pairs<V: Visitor<'de>>(
        entries: &[(Buffered<'de>, Buffered<'de>)],
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut map = MapDeserializer::new(
            entries.iter().map(|(key, value)| (PairKey(Ref::<E>::new(key)), Ref::<E>::new(value))),
        );
        let value = visitor.visit_map(&mut map)?;
        map.end()?;
        Ok(value)
    }

    fn visit_str<V: Visitor<'de>>(value: &Cow<'de, str>, visitor: V) -> Result<V::Value, E> {
        match value {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => visitor.visit_str(value),
        }
    }

    fn visit_bytes<V: Visitor<'de>>(value: &Cow<'de, [u8]>, visitor: V) -> Result<V::Value, E> {
        match value {
            Cow::Borrowed(value) => visitor.visit_borrowed_bytes(value),
            Cow::Owned(value) => visitor.visit_bytes(value),
        }
    }
}

macro_rules! deserialize_integer {
    ($($method:ident)*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
            match self.value {
                Buffered::U8(value) => visitor.visit_u8(*value),
                Buffered::U16(value) => visitor.visit_u16(*value),
                Buffered::U32(value) => visitor.visit_u32(*value),
                Buffered::U64(value) => visitor.visit_u64(*value),
                Buffered::I8(value) => visitor.visit_i8(*value),
                Buffered::I16(value) => visitor.visit_i16(*value),
                Buffered::I32(value) => visitor.visit_i32(*value),
                Buffered::I64(value) => visitor.visit_i64(*value),
                _ => Err(self.invalid_type(&visitor)),
            }
        }
    )*};
}

macro_rules! deserialize_float {
    ($($method:ident)*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
            match self.value {
                Buffered::F32(value) => visitor.visit_f32(*value),
                Buffered::F64(value) => visitor.visit_f64(*value),
                Buffered::U8(value) => visitor.visit_u8(*value),
                Buffered::U16(value) => visitor.visit_u16(*value),
                Buffered::U32(value) => visitor.visit_u32(*value),
                Buffered::U64(value) => visitor.visit_u64(*value),
                Buffered::I8(value) => visitor.visit_i8(*value),
                Buffered::I16(value) => visitor.visit_i16(*value),
                Buffered::I32(value) => visitor.visit_i32(*value),
                Buffered::I64(value) => visitor.visit_i64(*value),
                _ => Err(self.invalid_type(&visitor)),
            }
        }
    )*};
}

macro_rules! deserialize_string {
    ($($method:ident)*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
            match self.value {
                Buffered::Str(value) => Self::visit_str(value, visitor),
                Buffered::Bytes(value) => Self::visit_bytes(value, visitor),
                _ => Err(self.invalid_type(&visitor)),
            }
        }
    )*};
}

impl<'de, E: de::Error> Deserializer<'de> for Ref<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Null => visitor.visit_unit(),
            Buffered::None => visitor.visit_none(),
            Buffered::Some(value) => visitor.visit_some(Self::with_taken(value, self.taken)),
            Buffered::Newtype(value) => {
                visitor.visit_newtype_struct(Self::with_taken(value, self.taken))
            }
            Buffered::Bool(value) => visitor.visit_bool(*value),
            Buffered::U8(value) => visitor.visit_u8(*value),
            Buffered::U16(value) => visitor.visit_u16(*value),
            Buffered::U32(value) => visitor.visit_u32(*value),
            Buffered::U64(value) => visitor.visit_u64(*value),
            Buffered::I8(value) => visitor.visit_i8(*value),
            Buffered::I16(value) => visitor.visit_i16(*value),
            Buffered::I32(value) => visitor.visit_i32(*value),
            Buffered::I64(value) => visitor.visit_i64(*value),
            Buffered::F32(value) => visitor.visit_f32(*value),
            Buffered::F64(value) => visitor.visit_f64(*value),
            Buffered::Char(value) => visitor.visit_char(*value),
            Buffered::Str(value) => Self::visit_str(value, visitor),
            Buffered::Bytes(value) => Self::visit_bytes(value, visitor),
            Buffered::Seq(items) => Self::visit_seq(items, self.taken, visitor),
            Buffered::Map(entries) => Self::visit_map(entries, self.taken, visitor),
            Buffered::Pairs(entries) => Self::visit_pairs(entries, visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Bool(value) => visitor.visit_bool(*value),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    deserialize_integer! {
        deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64
        deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64
    }

    deserialize_float! { deserialize_f32 deserialize_f64 }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Char(value) => visitor.visit_char(*value),
            Buffered::Str(value) => Self::visit_str(value, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    deserialize_string! { deserialize_str deserialize_string }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Str(value) => Self::visit_str(value, visitor),
            Buffered::Bytes(value) => Self::visit_bytes(value, visitor),
            Buffered::Seq(items) => Self::visit_seq(items, self.taken, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::None => visitor.visit_none(),
            Buffered::Some(value) => visitor.visit_some(Self::with_taken(value, self.taken)),
            Buffered::Null => visitor.visit_unit(),
            // JSON does not say whether a value is optional.
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Null => visitor.visit_unit(),
            Buffered::Map(entries) if self.taken && entries.is_empty() => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, E> {
        if !self.taken {
            return self.deserialize_unit(visitor);
        }
        match self.value {
            Buffered::Map(entries) if entries.is_empty() => visitor.visit_unit(),
            Buffered::Seq(items) if items.is_empty() => visitor.visit_unit(),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, E> {
        // JSON does not say whether a value is a newtype. An input that does is read by serde's
        // own flattening.
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Seq(items) => Self::visit_seq(items, self.taken, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, E> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, E> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Map(entries) => Self::visit_map(entries, self.taken, visitor),
            Buffered::Pairs(entries) => Self::visit_pairs(entries, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        match self.value {
            Buffered::Seq(items) => Self::visit_seq(items, self.taken, visitor),
            Buffered::Map(entries) => Self::visit_map(entries, self.taken, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        let (variant, value) = match self.value {
            Buffered::Map(entries) => match entries.as_slice() {
                [(variant, value)] => (variant, Some(value)),
                _ => {
                    return Err(E::invalid_value(Unexpected::Map, &"map with a single key"));
                }
            },
            Buffered::Str(variant) => (variant, None),
            other => return Err(E::invalid_type(other.unexpected(), &"string or map")),
        };
        visitor.visit_enum(EnumRef { variant, value, taken: self.taken, marker: PhantomData })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Buffered::Str(value) => Self::visit_str(value, visitor),
            Buffered::Bytes(value) => Self::visit_bytes(value, visitor),
            // serde names a field or variant by its position with these two widths only.
            Buffered::U8(value) => visitor.visit_u8(*value),
            Buffered::U64(value) => visitor.visit_u64(*value),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }
}

impl<'de, E: de::Error> IntoDeserializer<'de, E> for Ref<'_, 'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Reads a key of [`Buffered::Pairs`] as the input handed it out, whatever is asked for. Its only
/// reader is serde's own flattening, which asks the input and not a buffer.
struct PairKey<'a, 'de, E>(Ref<'a, 'de, E>);

impl<'de, E: de::Error> Deserializer<'de> for PairKey<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.0.deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

impl<'de, E: de::Error> IntoDeserializer<'de, E> for PairKey<'_, 'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Reads an enum from a buffered string or single-entry map.
struct EnumRef<'a, 'de, E> {
    variant: &'a Cow<'de, str>,
    value: Option<&'a Buffered<'de>>,
    taken: bool,
    marker: PhantomData<E>,
}

impl<'a, 'de, E: de::Error> EnumAccess<'de> for EnumRef<'a, 'de, E> {
    type Error = E;
    type Variant = VariantRef<'a, 'de, E>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), E> {
        let variant = seed.deserialize(KeyRef(self.variant, PhantomData))?;
        Ok((variant, VariantRef { value: self.value, taken: self.taken, marker: PhantomData }))
    }
}

struct VariantRef<'a, 'de, E> {
    value: Option<&'a Buffered<'de>>,
    taken: bool,
    marker: PhantomData<E>,
}

impl<'de, E: de::Error> VariantAccess<'de> for VariantRef<'_, 'de, E> {
    type Error = E;

    fn unit_variant(self) -> Result<(), E> {
        match self.value {
            Some(value) => Deserialize::deserialize(Ref::with_taken(value, self.taken)),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, E> {
        match self.value {
            Some(value) => seed.deserialize(Ref::with_taken(value, self.taken)),
            None => Err(E::invalid_type(Unexpected::UnitVariant, &"newtype variant")),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Some(Buffered::Seq(items)) => Ref::<E>::visit_seq(items, self.taken, visitor),
            Some(other) => Err(E::invalid_type(other.unexpected(), &"tuple variant")),
            None => Err(E::invalid_type(Unexpected::UnitVariant, &"tuple variant")),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        match self.value {
            Some(Buffered::Map(entries)) => Ref::<E>::visit_map(entries, self.taken, visitor),
            Some(Buffered::Seq(items)) => Ref::<E>::visit_seq(items, self.taken, visitor),
            Some(other) => Err(E::invalid_type(other.unexpected(), &"struct variant")),
            None => Err(E::invalid_type(Unexpected::UnitVariant, &"struct variant")),
        }
    }
}

/// Reads the inner value from the entries, the way serde reads a flattened field: a struct takes
/// the entries it names, a map sees all entries and leaves them in place.
pub(super) struct Flat<'a, 'de, E>(&'a mut Entries<'de>, PhantomData<E>);

impl<'a, 'de, E> Flat<'a, 'de, E> {
    pub(super) const fn new(entries: &'a mut Entries<'de>) -> Self {
        Self(entries, PhantomData)
    }
}

macro_rules! only_structs_and_maps {
    ($($method:ident($($arg:ident: $ty:ty),*))*) => {$(
        fn $method<V: Visitor<'de>>(self, $($arg: $ty,)* _visitor: V) -> Result<V::Value, E> {
            $(let _ = $arg;)*
            Err(E::custom("can only flatten structs and maps"))
        }
    )*};
}

impl<'de, E: de::Error> Deserializer<'de> for Flat<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_map(AllEntries {
            entries: self.0.entries.iter(),
            value: None,
            marker: PhantomData,
        })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_map(NamedEntries {
            entries: self.0.entries.iter_mut(),
            fields,
            value: None,
            marker: PhantomData,
        })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_option<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, E> {
        // serde turns every error of a flattened `Option` into `None`, through a hook of its own
        // visitor that is not public.
        self.0.asked_for_option = true;
        Err(E::custom("a flattened option needs serde's own flattening"))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        for entry in &mut self.0.entries {
            if entry.as_ref().is_some_and(|(key, _)| variants.contains(&&**key)) {
                let (variant, value) = entry.take().expect("checked above");
                return visitor.visit_enum(EnumRef {
                    variant: &variant,
                    value: Some(&value),
                    taken: true,
                    marker: PhantomData,
                });
            }
        }
        Err(E::custom(format_args!("no variant of enum {name} found in flattened data")))
    }

    only_structs_and_maps! {
        deserialize_bool() deserialize_i8() deserialize_i16() deserialize_i32() deserialize_i64()
        deserialize_u8() deserialize_u16() deserialize_u32() deserialize_u64() deserialize_f32()
        deserialize_f64() deserialize_char() deserialize_str() deserialize_string()
        deserialize_bytes() deserialize_byte_buf() deserialize_seq() deserialize_identifier()
        deserialize_tuple(len: usize) deserialize_tuple_struct(name: &'static str, len: usize)
    }
}

/// All entries that are left, by reference.
struct AllEntries<'a, 'de, E> {
    entries: core::slice::Iter<'a, Option<(Cow<'de, str>, Buffered<'de>)>>,
    value: Option<&'a Buffered<'de>>,
    marker: PhantomData<E>,
}

impl<'de, E: de::Error> MapAccess<'de> for AllEntries<'_, 'de, E> {
    type Error = E;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, E> {
        let Some((key, value)) = self.entries.by_ref().flatten().next() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(KeyRef(key, PhantomData)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, E> {
        match self.value.take() {
            Some(value) => seed.deserialize(Ref::new(value)),
            None => Err(E::custom("value is missing")),
        }
    }
}

/// The entries with one of the given names, which are taken out.
struct NamedEntries<'a, 'de, E> {
    entries: core::slice::IterMut<'a, Option<(Cow<'de, str>, Buffered<'de>)>>,
    fields: &'static [&'static str],
    value: Option<Buffered<'de>>,
    marker: PhantomData<E>,
}

impl<'de, E: de::Error> MapAccess<'de> for NamedEntries<'_, 'de, E> {
    type Error = E;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, E> {
        for entry in self.entries.by_ref() {
            if entry.as_ref().is_some_and(|(key, _)| self.fields.contains(&&**key)) {
                let (key, value) = entry.take().expect("checked above");
                self.value = Some(value);
                return seed.deserialize(KeyRef(&key, PhantomData)).map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, E> {
        match self.value.take() {
            Some(value) => seed.deserialize(Ref::taken(&value)),
            None => Err(E::custom("value is missing")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::Error;

    /// A sequence or map that states a length and holds nothing.
    struct Stated(usize);

    impl<'de> SeqAccess<'de> for Stated {
        type Error = Error;

        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            _seed: T,
        ) -> Result<Option<T::Value>, Error> {
            Ok(None)
        }

        fn size_hint(&self) -> Option<usize> {
            Some(self.0)
        }
    }

    impl<'de> MapAccess<'de> for Stated {
        type Error = Error;

        fn next_key_seed<K: DeserializeSeed<'de>>(
            &mut self,
            _seed: K,
        ) -> Result<Option<K::Value>, Error> {
            Ok(None)
        }

        fn next_value_seed<V: DeserializeSeed<'de>>(
            &mut self,
            _seed: V,
        ) -> Result<V::Value, Error> {
            Err(de::Error::custom("value is missing"))
        }

        fn size_hint(&self) -> Option<usize> {
            Some(self.0)
        }
    }

    /// The capacities the buffer reserves for a sequence, a map and an object of `len` elements.
    fn reserved(len: usize) -> [usize; 3] {
        let flag = Cell::new(false);
        let Ok(Buffered::Seq(items)) = BufferedSeed(&flag).visit_seq(Stated(len)) else {
            panic!("no sequence")
        };
        let Ok(Buffered::Map(entries)) = BufferedSeed(&flag).visit_map(Stated(len)) else {
            panic!("no map")
        };
        let object = Entries::read(Stated(len)).unwrap();
        [items.capacity(), entries.capacity(), object.entries.capacity()]
    }

    #[test]
    fn reserves_the_stated_length() {
        assert!(reserved(100).iter().all(|capacity| *capacity >= 100));

        // The input may state any length.
        const MAX_BYTES: usize = 1024 * 1024;
        let [items, entries, object] = reserved(usize::MAX);
        assert_eq!(items, MAX_BYTES / size_of::<Buffered<'_>>());
        assert_eq!(entries, MAX_BYTES / size_of::<(Cow<'_, str>, Buffered<'_>)>());
        assert_eq!(object, MAX_BYTES / size_of::<Option<(Cow<'_, str>, Buffered<'_>)>>());
    }
}
