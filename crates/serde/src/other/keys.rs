//! A [`Serializer`] that removes the top-level keys of a value from a map without serializing the
//! values.

use alloc::{collections::BTreeMap, string::String};
use serde::{
    ser::{
        Error as _, Impossible, SerializeMap, SerializeSeq, SerializeStruct,
        SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize, Serializer,
};
use serde_json::{Error, Value};

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
) -> Result<(), Error> {
    value.serialize(KeyRemover(map))
}

struct KeyRemover<'a>(&'a mut BTreeMap<String, Value>);

macro_rules! ignore {
    ($($method:ident($($ty:ty)?))*) => {$(
        fn $method(self, $(_: $ty)?) -> Result<(), Error> {
            Ok(())
        }
    )*};
}

impl Serializer for KeyRemover<'_> {
    type Ok = ();
    type Error = Error;
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

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Error> {
        value.serialize(self)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        Ok(())
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Error> {
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        self.0.remove(variant);
        Ok(())
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Ignore, Error> {
        Ok(Ignore)
    }

    fn serialize_tuple(self, _: usize) -> Result<Ignore, Error> {
        Ok(Ignore)
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Ignore, Error> {
        Ok(Ignore)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Ignore, Error> {
        self.0.remove(variant);
        Ok(Ignore)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self, Error> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Error> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Ignore, Error> {
        self.0.remove(variant);
        Ok(Ignore)
    }
}

impl SerializeStruct for KeyRemover<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        self.0.remove(key);
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializeMap for KeyRemover<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        key.serialize(KeyCapture(self.0))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

/// Serializes nothing: the keys of nested values are not top-level keys.
struct Ignore;

impl SerializeSeq for Ignore {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializeTuple for Ignore {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializeTupleStruct for Ignore {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializeTupleVariant for Ignore {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializeStructVariant for Ignore {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

/// Removes a string map key from the map and rejects every other key type.
struct KeyCapture<'a>(&'a mut BTreeMap<String, Value>);

macro_rules! reject {
    ($($method:ident($($ty:ty)?))*) => {$(
        fn $method(self, $(_: $ty)?) -> Result<(), Error> {
            Err(Error::custom("map key is not a string"))
        }
    )*};
}

impl Serializer for KeyCapture<'_> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_str(self, key: &str) -> Result<(), Error> {
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

    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Error> {
        Err(Error::custom("map key is not a string"))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error::custom("map key is not a string"))
    }
}
