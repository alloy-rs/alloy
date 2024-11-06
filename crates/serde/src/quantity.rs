//! Serde functions for encoding primitive numbers using the Ethereum JSON-RPC "quantity" format.
//!
//! This is defined as a "hex encoded unsigned integer", with a special case of 0 being `0x0`.
//!
//! A regex for this format is: `^0x([1-9a-f]+[0-9a-f]*|0)$`.
//!
//! This is only valid for human-readable [`serde`] implementations.
//! For non-human-readable implementations, the format is unspecified.
//! Currently, it uses a fixed-width big-endian byte-array.

use private::ConvertRuint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes a primitive number as a "quantity" hex string.
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: ConvertRuint,
    S: Serializer,
{
    value.into_ruint().serialize(serializer)
}

/// Deserializes a primitive number from a "quantity" hex string.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: ConvertRuint,
    D: Deserializer<'de>,
{
    T::Ruint::deserialize(deserializer).map(T::from_ruint)
}

/// Serde functions for encoding optional primitive numbers using the Ethereum "quantity" format.
///
/// See [`quantity`](self) for more information.
pub mod opt {
    use super::private::ConvertRuint;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serializes an optional primitive number as a "quantity" hex string.
    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: ConvertRuint,
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.into_ruint()),
            None => serializer.serialize_none(),
        }
    }

    /// Deserializes an optional primitive number from a "quantity" hex string.
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: ConvertRuint,
        D: Deserializer<'de>,
    {
        Ok(Option::<T::Ruint>::deserialize(deserializer)?.map(T::from_ruint))
    }
}

/// Serde functions for encoding a list of primitive numbers using the Ethereum "quantity" format.
///
/// See [`quantity`](self) for more information.
pub mod vec {
    use super::private::ConvertRuint;
    use alloc::vec::Vec;
    use core::{fmt, marker::PhantomData};
    use serde::{
        de::{SeqAccess, Visitor},
        ser::SerializeSeq,
        Deserializer, Serializer,
    };

    /// Serializes a vector of primitive numbers as a "quantity" hex string.
    pub fn serialize<T, S>(value: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        T: ConvertRuint,
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for val in value {
            seq.serialize_element(&val.into_ruint())?;
        }
        seq.end()
    }

    /// Deserializes a vector of primitive numbers from a "quantity" hex string.
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        T: ConvertRuint,
        D: Deserializer<'de>,
    {
        struct VecVisitor<T> {
            marker: PhantomData<T>,
        }

        impl<'de, T> Visitor<'de> for VecVisitor<T>
        where
            T: ConvertRuint,
        {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::<T>::with_capacity(seq.size_hint().unwrap_or(0));

                while let Some(value) = seq.next_element::<T::Ruint>()? {
                    values.push(T::from_ruint(value));
                }
                Ok(values)
            }
        }

        let visitor = VecVisitor { marker: PhantomData };
        deserializer.deserialize_seq(visitor)
    }
}

/// serde functions for handling `Vec<Vec<u128>>` via [U128](alloy_primitives::U128)
pub mod u128_vec_vec_opt {
    use alloy_primitives::U128;
    use serde::{Deserialize, Deserializer, Serializer};

    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    /// Deserializes an `u128` accepting a hex quantity string with optional 0x prefix or
    /// a number
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Vec<u128>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<Vec<U128>>>::deserialize(deserializer)?.map_or_else(
            || Ok(None),
            |vec| {
                Ok(Some(
                    vec.into_iter().map(|v| v.into_iter().map(|val| val.to()).collect()).collect(),
                ))
            },
        )
    }

    /// Serializes u128 as hex string
    pub fn serialize<S: Serializer>(
        value: &Option<Vec<Vec<u128>>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(vec) => {
                let vec = vec
                    .iter()
                    .map(|v| v.iter().map(|val| U128::from(*val)).collect::<Vec<_>>())
                    .collect::<Vec<_>>();
                s.serialize_some(&vec)
            }
            None => s.serialize_none(),
        }
    }
}

/// Private implementation details of the [`quantity`](self) module.
#[allow(unnameable_types)]
mod private {
    #[doc(hidden)]
    pub trait ConvertRuint: Copy + Sized {
        // We have to use `Try*` traits because `From` is not implemented by ruint types.
        // They shouldn't ever error.
        type Ruint: Copy
            + serde::Serialize
            + serde::de::DeserializeOwned
            + TryFrom<Self>
            + TryInto<Self>;

        #[inline]
        fn into_ruint(self) -> Self::Ruint {
            self.try_into().ok().unwrap()
        }

        #[inline]
        fn from_ruint(ruint: Self::Ruint) -> Self {
            ruint.try_into().ok().unwrap()
        }
    }

    macro_rules! impl_from_ruint {
        ($($primitive:ty = $ruint:ty),* $(,)?) => {
            $(
                impl ConvertRuint for $primitive {
                    type Ruint = $ruint;
                }
            )*
        };
    }

    impl_from_ruint! {
        bool = alloy_primitives::ruint::aliases::U1,
        u8   = alloy_primitives::U8,
        u16  = alloy_primitives::U16,
        u32  = alloy_primitives::U32,
        u64  = alloy_primitives::U64,
        u128 = alloy_primitives::U128,
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[cfg(not(feature = "std"))]
    use alloc::{string::ToString, vec, vec::Vec};

    #[test]
    fn test_hex_u64() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "super")]
            inner: u64,
        }

        let val = Value { inner: 1000 };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "super")]
            inner: u128,
        }

        let val = Value { inner: 1000 };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let s = "{\"inner\":\"1000\"}".to_string();
        let deserialized: Value = serde_json::from_str(&s).unwrap();

        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_opt_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "super::opt")]
            inner: Option<u128>,
        }

        let val = Value { inner: Some(1000) };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":\"0x3e8\"}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let s = "{\"inner\":\"1000\"}".to_string();
        let deserialized: Value = serde_json::from_str(&s).unwrap();

        assert_eq!(val, deserialized);

        let val = Value { inner: None };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":null}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_vec_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "super::vec")]
            inner: Vec<u128>,
        }

        let val = Value { inner: vec![1000, 2000] };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":[\"0x3e8\",\"0x7d0\"]}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_u128_vec_vec_opt_via_ruint() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Value {
            #[serde(with = "super::u128_vec_vec_opt")]
            inner: Option<Vec<Vec<u128>>>,
        }

        let val = Value { inner: Some(vec![vec![1000, 2000], vec![3000, 4000]]) };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":[[\"0x3e8\",\"0x7d0\"],[\"0xbb8\",\"0xfa0\"]]}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);

        let val = Value { inner: None };
        let s = serde_json::to_string(&val).unwrap();
        assert_eq!(s, "{\"inner\":null}");

        let deserialized: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(val, deserialized);
    }
}
