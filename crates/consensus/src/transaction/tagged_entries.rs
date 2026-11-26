//! The deserializer the [`TransactionEnvelope`](crate::TransactionEnvelope) derive hands a variant
//! whose tag it read from the first key of an object.

use serde::{
    de::{value::MapAccessDeserializer, MapAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};

/// The entries of a transaction object that follow its leading `type`.
///
/// This answers the hints of a variant the way serde's tagged enums do: a newtype struct and an
/// `Option` read the entries as their content, where [`MapAccessDeserializer`] rejects them.
#[doc(hidden)]
#[derive(Debug)]
pub struct TaggedEntries<M>(pub M);

impl<'de, M: MapAccess<'de>> Deserializer<'de> for TaggedEntries<M> {
    type Error = M::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, M::Error> {
        visitor.visit_map(self.0)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, M::Error> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, M::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, M::Error> {
        MapAccessDeserializer::new(self.0).deserialize_enum(name, variants, visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{
        de::value::{Error, MapDeserializer},
        Deserialize,
    };

    #[test]
    fn answers_hints_like_a_tagged_enum() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Fields {
            a: u64,
        }

        #[derive(Debug, PartialEq, Deserialize)]
        struct Newtype(Fields);

        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "camelCase")]
        enum Enum {
            A(u64),
        }

        let entries = || TaggedEntries(MapDeserializer::<_, Error>::new([("a", 1u64)].into_iter()));

        assert_eq!(Fields::deserialize(entries()), Ok(Fields { a: 1 }));
        assert_eq!(Newtype::deserialize(entries()), Ok(Newtype(Fields { a: 1 })));
        assert_eq!(Option::deserialize(entries()), Ok(Some(Fields { a: 1 })));
        assert_eq!(Enum::deserialize(entries()), Ok(Enum::A(1)));
    }
}
