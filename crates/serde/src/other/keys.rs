//! A [`Serializer`] that removes the top-level keys of a value from a map without serializing the
//! values.

use alloc::{collections::BTreeMap, string::String};
use core::fmt;
use serde::{
    ser::{
        Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
        SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, StdError,
    },
    Serialize, Serializer,
};
use serde_json::Value;

/// Removes the top-level keys that `value` serializes to from `map`.
///
/// This removes the same keys as `serde_json::to_value(value)` would produce, but never
/// serializes the values.
///
/// Returns an error if a map key is not a string. Keys removed up to that point are keys of
/// `value`.
pub(super) fn remove_keys<T: Serialize>(
    value: &T,
    map: &mut BTreeMap<String, Value>,
) -> Result<(), UnsupportedKey> {
    value.serialize(KeyRemover(map))
}

/// Error for map keys that are not strings.
#[derive(Debug)]
pub(super) struct UnsupportedKey;

impl fmt::Display for UnsupportedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("map key is not a string")
    }
}

impl StdError for UnsupportedKey {}

impl serde::ser::Error for UnsupportedKey {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Self
    }
}

struct KeyRemover<'a>(&'a mut BTreeMap<String, Value>);

macro_rules! ignore {
    ($($method:ident($($ty:ty)?))*) => {$(
        fn $method(self, $(_: $ty)?) -> Result<(), UnsupportedKey> {
            Ok(())
        }
    )*};
}

impl<'a> Serializer for KeyRemover<'a> {
    type Ok = ();
    type Error = UnsupportedKey;
    type SerializeSeq = Ignore;
    type SerializeTuple = Ignore;
    type SerializeTupleStruct = Ignore;
    type SerializeTupleVariant = Ignore;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Ignore;

    ignore! {
        serialize_bool(bool) serialize_i8(i8) serialize_i16(i16) serialize_i32(i32)
        serialize_i64(i64) serialize_i128(i128) serialize_u8(u8) serialize_u16(u16)
        serialize_u32(u32) serialize_u64(u64) serialize_u128(u128) serialize_f32(f32)
        serialize_f64(f64) serialize_char(char) serialize_str(&str) serialize_bytes(&[u8])
        serialize_none() serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), UnsupportedKey> {
        value.serialize(self)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), UnsupportedKey> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: &T,
    ) -> Result<(), UnsupportedKey> {
        self.0.remove(variant);
        Ok(())
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Ignore, UnsupportedKey> {
        Ok(Ignore)
    }

    fn serialize_tuple(self, _: usize) -> Result<Ignore, UnsupportedKey> {
        Ok(Ignore)
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Ignore, UnsupportedKey> {
        Ok(Ignore)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Ignore, UnsupportedKey> {
        self.0.remove(variant);
        Ok(Ignore)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self, UnsupportedKey> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, UnsupportedKey> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Ignore, UnsupportedKey> {
        self.0.remove(variant);
        Ok(Ignore)
    }
}

impl SerializeStruct for KeyRemover<'_> {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        _: &T,
    ) -> Result<(), UnsupportedKey> {
        self.0.remove(key);
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

impl SerializeMap for KeyRemover<'_> {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), UnsupportedKey> {
        key.serialize(KeyCapture(self.0))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

/// Serializes nothing: the keys of nested values are not top-level keys.
struct Ignore;

impl SerializeSeq for Ignore {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

impl SerializeTuple for Ignore {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

impl SerializeTupleStruct for Ignore {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

impl SerializeTupleVariant for Ignore {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

impl SerializeStructVariant for Ignore {
    type Ok = ();
    type Error = UnsupportedKey;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        _: &T,
    ) -> Result<(), UnsupportedKey> {
        Ok(())
    }

    fn end(self) -> Result<(), UnsupportedKey> {
        Ok(())
    }
}

/// Removes a string map key from the map and rejects every other key type.
struct KeyCapture<'a>(&'a mut BTreeMap<String, Value>);

macro_rules! reject {
    ($($method:ident($($ty:ty)?))*) => {$(
        fn $method(self, $(_: $ty)?) -> Result<(), UnsupportedKey> {
            Err(UnsupportedKey)
        }
    )*};
}

impl Serializer for KeyCapture<'_> {
    type Ok = ();
    type Error = UnsupportedKey;
    type SerializeSeq = Impossible<(), UnsupportedKey>;
    type SerializeTuple = Impossible<(), UnsupportedKey>;
    type SerializeTupleStruct = Impossible<(), UnsupportedKey>;
    type SerializeTupleVariant = Impossible<(), UnsupportedKey>;
    type SerializeMap = Impossible<(), UnsupportedKey>;
    type SerializeStruct = Impossible<(), UnsupportedKey>;
    type SerializeStructVariant = Impossible<(), UnsupportedKey>;

    fn serialize_str(self, key: &str) -> Result<(), UnsupportedKey> {
        self.0.remove(key);
        Ok(())
    }

    reject! {
        serialize_bool(bool) serialize_i8(i8) serialize_i16(i16) serialize_i32(i32)
        serialize_i64(i64) serialize_i128(i128) serialize_u8(u8) serialize_u16(u16)
        serialize_u32(u32) serialize_u64(u64) serialize_u128(u128) serialize_f32(f32)
        serialize_f64(f64) serialize_char(char) serialize_bytes(&[u8])
        serialize_none() serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<(), UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, UnsupportedKey> {
        Err(UnsupportedKey)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, UnsupportedKey> {
        Err(UnsupportedKey)
    }
}
