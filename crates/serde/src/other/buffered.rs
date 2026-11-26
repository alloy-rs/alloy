//! A borrowed buffer for an object that is decoded twice: once into the inner value of a
//! [`WithOtherFields`](super::WithOtherFields), and once into the fields that value did not use.
//!
//! This mirrors what serde's `#[serde(flatten)]` does with its private `Content` buffer, which
//! does not let the second reader skip the entries the first one used. [`Flat`] and [`Ref`] answer
//! every hint the way serde's `FlatMapDeserializer` and `ContentRefDeserializer` do.

use super::keys::KeySet;
use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::{cell::Cell, fmt, marker::PhantomData};
use serde::{
    de::{
        self,
        value::{MapDeserializer, SeqDeserializer},
        DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, Unexpected,
        VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};
use serde_json::Value;

/// A buffered value. Strings borrow from the input where the input allows it.
#[derive(Debug)]
pub(super) enum Buffered<'de> {
    Null,
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(f64),
    Str(Cow<'de, str>),
    Bytes(Cow<'de, [u8]>),
    Seq(Vec<Self>),
    Map(Vec<(Cow<'de, str>, Self)>),
}

impl Buffered<'_> {
    fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Self::Null => Unexpected::Unit,
            Self::Bool(value) => Unexpected::Bool(*value),
            Self::U64(value) => Unexpected::Unsigned(*value),
            Self::I64(value) => Unexpected::Signed(*value),
            Self::F64(value) => Unexpected::Float(*value),
            Self::Str(value) => Unexpected::Str(value),
            Self::Bytes(value) => Unexpected::Bytes(value),
            Self::Seq(_) => Unexpected::Seq,
            Self::Map(_) => Unexpected::Map,
        }
    }
}

/// Buffers a value and notes whether it holds a byte string.
#[derive(Clone, Copy)]
struct BufferedSeed<'a>(&'a Cell<bool>);

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

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Buffered::U64(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Buffered::I64(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Buffered::F64(value))
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
        Ok(Buffered::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(self)? {
            items.push(item);
        }
        Ok(Buffered::Seq(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut entries = Vec::new();
        while let Some(key) = map.next_key_seed(KeySeed)? {
            entries.push((key, map.next_value_seed(self)?));
        }
        Ok(Buffered::Map(entries))
    }
}

/// Reads a map key, borrowing it where the input allows it.
struct KeySeed;

impl<'de> DeserializeSeed<'de> for KeySeed {
    type Value = Cow<'de, str>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct KeyVisitor;

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Cow<'de, str>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string key")
            }

            fn visit_borrowed_str<E>(self, key: &'de str) -> Result<Self::Value, E> {
                Ok(Cow::Borrowed(key))
            }

            fn visit_str<E>(self, key: &str) -> Result<Self::Value, E> {
                Ok(Cow::Owned(key.to_string()))
            }

            fn visit_string<E>(self, key: String) -> Result<Self::Value, E> {
                Ok(Cow::Owned(key))
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}

/// The entries of the buffered object. An entry is `None` once a reader took it.
#[derive(Debug)]
pub(super) struct Entries<'de> {
    entries: Vec<Option<(Cow<'de, str>, Buffered<'de>)>>,
    has_byte_strings: bool,
    asked_for_option: bool,
}

impl<'de> Entries<'de> {
    /// Buffers the entries of `map`.
    pub(super) fn read<A: MapAccess<'de>>(mut map: A) -> Result<Self, A::Error> {
        let has_byte_strings = Cell::new(false);
        let mut entries = Vec::new();
        while let Some(key) = map.next_key_seed(KeySeed)? {
            entries.push(Some((key, map.next_value_seed(BufferedSeed(&has_byte_strings))?)));
        }
        Ok(Self { entries, has_byte_strings: has_byte_strings.get(), asked_for_option: false })
    }

    /// Whether a value holds a byte string, which no other field can hold.
    pub(super) const fn has_byte_strings(&self) -> bool {
        self.has_byte_strings
    }

    /// Whether the inner value asked [`Flat`] for an `Option`, which it cannot answer.
    pub(super) const fn asked_for_option(&self) -> bool {
        self.asked_for_option
    }

    /// Reads the entries that are left as a map.
    pub(super) fn deserializer<'a, E: de::Error + 'a>(
        &'a self,
    ) -> impl Deserializer<'de, Error = E> + 'a
    where
        'de: 'a,
    {
        MapDeserializer::new(
            self.entries
                .iter()
                .flatten()
                .map(|(key, value)| (KeyRef(key, PhantomData), Ref::<E>::new(value))),
        )
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

/// Reads a buffered key.
struct KeyRef<'a, 'de, E>(&'a Cow<'de, str>, PhantomData<E>);

impl<'de, E: de::Error> Deserializer<'de> for KeyRef<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.0 {
            Cow::Borrowed(key) => visitor.visit_borrowed_str(key),
            Cow::Owned(key) => visitor.visit_str(key),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
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
    const fn new(value: &'a Buffered<'de>) -> Self {
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
                Buffered::U64(value) => visitor.visit_u64(*value),
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
                Buffered::F64(value) => visitor.visit_f64(*value),
                Buffered::U64(value) => visitor.visit_u64(*value),
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
            Buffered::Bool(value) => visitor.visit_bool(*value),
            Buffered::U64(value) => visitor.visit_u64(*value),
            Buffered::I64(value) => visitor.visit_i64(*value),
            Buffered::F64(value) => visitor.visit_f64(*value),
            Buffered::Str(value) => Self::visit_str(value, visitor),
            Buffered::Bytes(value) => Self::visit_bytes(value, visitor),
            Buffered::Seq(items) => Self::visit_seq(items, self.taken, visitor),
            Buffered::Map(entries) => Self::visit_map(entries, self.taken, visitor),
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
            Buffered::Null => visitor.visit_unit(),
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
