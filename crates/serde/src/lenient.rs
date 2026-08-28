//! Serde functions for leniently deserializing JSON-RPC request parameters.
//!
//! JSON-RPC clients are inconsistent in how they encode integers in a request's `params`. The same
//! block number can arrive as a JSON number (`100`), as a quantity string (`"0x64"`), or as a
//! decimal string (`"100"`). On top of that, a single positional parameter can arrive either bare
//! (`100`) or wrapped in the one-element sequence the JSON-RPC specification prescribes (`[100]`).
//!
//! Servers built on alloy types have to accept all of these shapes. The helpers here do that:
//!
//! | Function | `100` | `"0x64"` | `"100"` | `[100]` | `null` | `[]`, `[null]` |
//! |---|---|---|---|---|---|---|
//! | [`deserialize`] | yes | yes | yes | no | no | no |
//! | [`opt::deserialize`] | yes | yes | yes | no | yes | no |
//! | [`seq::deserialize`] | yes | yes | yes | yes | no | no |
//! | [`seq::opt::deserialize`] | yes | yes | yes | yes | yes | yes |
//!
//! Sequences of more than one element are rejected, as is any value that does not fit the target
//! type: overflow and malformed input are always errors, never silent truncation.
//!
//! The helpers support `u8` through `u128` and [`Uint`](alloy_primitives::Uint) types such as
//! [`U256`](alloy_primitives::U256). They are deserializers only; to serialize, use the
//! [`quantity`](crate::quantity) module, whose deserializer already accepts the three scalar
//! shapes for primitive integers. What this module adds is a single generic entry point that also
//! covers `Uint` targets, and the sequence handling of [`seq`].
//!
//! This is only valid for self-describing, human-readable [`serde`] implementations.

use private::LenientNumber;
use serde::Deserializer;

/// Deserializes a number from a JSON number, a `0x`-prefixed hex string, or a decimal string.
///
/// ```
/// # use alloy_primitives::U256;
/// # use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct Params {
///     #[serde(deserialize_with = "alloy_serde::lenient::deserialize")]
///     value: U256,
/// }
///
/// for raw in [r#"{"value":100}"#, r#"{"value":"0x64"}"#, r#"{"value":"100"}"#] {
///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().value, U256::from(100));
/// }
/// ```
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: LenientNumber,
    D: Deserializer<'de>,
{
    T::deserialize_lenient(deserializer)
}

/// Serde functions for leniently deserializing optional numbers.
///
/// See [`lenient`](crate::lenient) for more information.
pub mod opt {
    use super::private::{Lenient, LenientNumber};
    use serde::{Deserialize, Deserializer};

    /// Deserializes an optional number from `null`, a JSON number, a `0x`-prefixed hex string, or
    /// a decimal string.
    ///
    /// ```
    /// # use serde::Deserialize;
    /// #[derive(Deserialize)]
    /// struct Params {
    ///     #[serde(default, deserialize_with = "alloy_serde::lenient::opt::deserialize")]
    ///     value: Option<u64>,
    /// }
    ///
    /// for raw in [r#"{"value":100}"#, r#"{"value":"0x64"}"#, r#"{"value":"100"}"#] {
    ///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().value, Some(100));
    /// }
    /// for raw in [r#"{"value":null}"#, "{}"] {
    ///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().value, None);
    /// }
    /// ```
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: LenientNumber,
        D: Deserializer<'de>,
    {
        Ok(Option::<Lenient<T>>::deserialize(deserializer)?.map(Lenient::into_inner))
    }
}

/// Serde functions for leniently deserializing a single number that may be wrapped in a
/// one-element sequence, which is the JSON-RPC positional parameter shape.
///
/// See [`lenient`](crate::lenient) for more information.
pub mod seq {
    use super::private::{LenientNumber, ValueOrSeq};
    use serde::Deserializer;

    /// Deserializes a single number from either a bare value or a one-element sequence: `100`,
    /// `"0x64"`, `"100"`, `[100]`, `["0x64"]` and `["100"]` all deserialize to `100`.
    ///
    /// Empty sequences, sequences with more than one element, and `null` are rejected.
    ///
    /// ```
    /// # use serde::Deserialize;
    /// #[derive(Deserialize)]
    /// struct Params(#[serde(deserialize_with = "alloy_serde::lenient::seq::deserialize")] u64);
    ///
    /// for raw in ["100", r#""0x64""#, "[100]", r#"["0x64"]"#] {
    ///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().0, 100);
    /// }
    /// for raw in ["[]", "[1, 2]", "null", r#""0x10000000000000000""#] {
    ///     assert!(serde_json::from_str::<Params>(raw).is_err());
    /// }
    /// ```
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: LenientNumber,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueOrSeq::new())
    }

    /// Serde functions for leniently deserializing an optional single number that may be wrapped
    /// in a one-element sequence.
    ///
    /// See [`lenient`](crate::lenient) for more information.
    pub mod opt {
        use super::super::private::{LenientNumber, OptionalValueOrSeq};
        use serde::Deserializer;

        /// Deserializes an optional single number from either a bare value or a sequence of at
        /// most one element: `[]`, `[null]` and `null` deserialize to `None`, while `100`,
        /// `"0x64"`, `[100]` and `["0x64"]` deserialize to `Some(100)`.
        ///
        /// Sequences with more than one element are rejected.
        ///
        /// ```
        /// # use serde::Deserialize;
        /// #[derive(Deserialize)]
        /// struct Params(
        ///     #[serde(default, deserialize_with = "alloy_serde::lenient::seq::opt::deserialize")]
        ///     Option<u64>,
        /// );
        ///
        /// for raw in ["100", "[100]", r#"["0x64"]"#, r#"["100"]"#] {
        ///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().0, Some(100));
        /// }
        /// for raw in ["[]", "[null]", "null"] {
        ///     assert_eq!(serde_json::from_str::<Params>(raw).unwrap().0, None);
        /// }
        /// assert!(serde_json::from_str::<Params>("[1, 2]").is_err());
        /// ```
        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            T: LenientNumber,
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(OptionalValueOrSeq::new())
        }
    }
}

/// Private implementation details of the [`lenient`](self) module.
#[expect(unnameable_types)]
mod private {
    use alloy_primitives::{Uint, U128, U16, U32, U64, U8};
    use core::{fmt, marker::PhantomData};
    use serde::{
        de::{Error, IgnoredAny, IntoDeserializer, SeqAccess, Visitor},
        Deserialize, Deserializer,
    };

    /// A number that can be deserialized from a JSON number, a `0x`-prefixed hex string, or a
    /// decimal string.
    #[doc(hidden)]
    pub trait LenientNumber: Sized {
        fn deserialize_lenient<'de, D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>;
    }

    impl<const BITS: usize, const LIMBS: usize> LenientNumber for Uint<BITS, LIMBS> {
        fn deserialize_lenient<'de, D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            Self::deserialize(deserializer)
        }
    }

    macro_rules! impl_lenient_number {
        ($($primitive:ty = $ruint:ty),* $(,)?) => {
            $(
                impl LenientNumber for $primitive {
                    fn deserialize_lenient<'de, D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        <$ruint>::deserialize(deserializer).map(|value| value.to::<Self>())
                    }
                }
            )*
        };
    }

    impl_lenient_number! {
        u8   = U8,
        u16  = U16,
        u32  = U32,
        u64  = U64,
        u128 = U128,
    }

    /// [`Deserialize`] adapter for [`LenientNumber`].
    pub(crate) struct Lenient<T>(T);

    impl<T> Lenient<T> {
        pub(crate) fn into_inner(self) -> T {
            self.0
        }
    }

    impl<'de, T: LenientNumber> Deserialize<'de> for Lenient<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            T::deserialize_lenient(deserializer).map(Self)
        }
    }

    /// Visitor for a number that may be wrapped in a one-element sequence.
    pub(crate) struct ValueOrSeq<T>(PhantomData<T>);

    impl<T> ValueOrSeq<T> {
        pub(crate) const fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<'de, T: LenientNumber> Visitor<'de> for ValueOrSeq<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(
                "a number, a hex or decimal string, or a sequence containing exactly one of them",
            )
        }

        fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer())
        }

        fn visit_u128<E: Error>(self, value: u128) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer())
        }

        fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer())
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let Some(value) = seq.next_element::<Lenient<T>>()? else {
                return Err(A::Error::invalid_length(0, &self));
            };
            match remaining(&mut seq)? {
                0 => Ok(value.into_inner()),
                extra => Err(A::Error::invalid_length(extra + 1, &self)),
            }
        }
    }

    /// Visitor for an optional number that may be wrapped in a sequence of at most one element.
    pub(crate) struct OptionalValueOrSeq<T>(PhantomData<T>);

    impl<T> OptionalValueOrSeq<T> {
        pub(crate) const fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<'de, T: LenientNumber> Visitor<'de> for OptionalValueOrSeq<T> {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(
                "null, a number, a hex or decimal string, or a sequence containing at most one of them",
            )
        }

        fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer()).map(Some)
        }

        fn visit_u128<E: Error>(self, value: u128) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer()).map(Some)
        }

        fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
            T::deserialize_lenient(value.into_deserializer()).map(Some)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let value = seq.next_element::<Option<Lenient<T>>>()?.flatten();
            match remaining(&mut seq)? {
                0 => Ok(value.map(Lenient::into_inner)),
                extra => Err(A::Error::invalid_length(extra + 1, &self)),
            }
        }
    }

    /// Drains the sequence and returns the number of elements that were left in it.
    fn remaining<'de, A: SeqAccess<'de>>(seq: &mut A) -> Result<usize, A::Error> {
        let mut extra = 0;
        while seq.next_element::<IgnoredAny>()?.is_some() {
            extra += 1;
        }
        Ok(extra)
    }
}

#[cfg(test)]
mod tests {
    use super::private::LenientNumber;
    use alloc::string::ToString;
    use alloy_primitives::U256;
    use serde::{de::IntoDeserializer, Deserialize};
    use serde_json::{json, Value};

    fn value<T: LenientNumber>(value: Value) -> Result<T, serde_json::Error> {
        super::deserialize(value.into_deserializer())
    }

    fn opt_value<T: LenientNumber>(value: Value) -> Result<Option<T>, serde_json::Error> {
        super::opt::deserialize(value.into_deserializer())
    }

    fn seq<T: LenientNumber>(value: Value) -> Result<T, serde_json::Error> {
        super::seq::deserialize(value.into_deserializer())
    }

    fn opt_seq<T: LenientNumber>(value: Value) -> Result<Option<T>, serde_json::Error> {
        super::seq::opt::deserialize(value.into_deserializer())
    }

    #[test]
    fn deserializes_number_hex_and_decimal_strings() {
        for raw in [json!(100), json!("0x64"), json!("100")] {
            assert_eq!(value::<u64>(raw.clone()).unwrap(), 100);
            assert_eq!(value::<U256>(raw.clone()).unwrap(), U256::from(100));
            assert_eq!(opt_value::<u64>(raw).unwrap(), Some(100));
        }

        assert_eq!(value::<u64>(json!(u64::MAX)).unwrap(), u64::MAX);
        assert_eq!(opt_value::<u64>(json!(null)).unwrap(), None);
    }

    #[test]
    fn rejects_values_that_do_not_fit_the_target_type() {
        for raw in [
            json!("0x10000000000000000"),
            json!("18446744073709551616"),
            json!(-1),
            json!(1.5),
            json!(null),
            json!([100]),
        ] {
            assert!(value::<u64>(raw).is_err());
        }

        // narrower types are not truncated
        assert_eq!(value::<u8>(json!(255)).unwrap(), 255);
        assert!(value::<u8>(json!(256)).is_err());
        assert!(value::<u8>(json!("0x100")).is_err());

        assert!(value::<U256>(json!(
            "0x10000000000000000000000000000000000000000000000000000000000000000"
        ))
        .is_err());
        assert!(value::<U256>(json!(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936"
        ))
        .is_err());
    }

    #[test]
    fn deserializes_single_param_sequence_and_bare_value() {
        for raw in
            [json!([100]), json!(100), json!(["0x64"]), json!("0x64"), json!(["100"]), json!("100")]
        {
            assert_eq!(seq::<u64>(raw.clone()).unwrap(), 100);
            assert_eq!(seq::<U256>(raw.clone()).unwrap(), U256::from(100));
            assert_eq!(opt_seq::<u64>(raw).unwrap(), Some(100));
        }

        for raw in [json!([u64::MAX]), json!(u64::MAX)] {
            assert_eq!(seq::<u64>(raw.clone()).unwrap(), u64::MAX);
            assert_eq!(opt_seq::<u64>(raw).unwrap(), Some(u64::MAX));
        }
    }

    #[test]
    fn rejects_invalid_sequence_shape_and_overflow() {
        for raw in [
            json!([]),
            json!([1, 2]),
            json!([null]),
            json!(null),
            json!([["0x64"]]),
            json!(["0x10000000000000000"]),
            json!("0x10000000000000000"),
            json!(["18446744073709551616"]),
            json!("18446744073709551616"),
        ] {
            assert!(seq::<u64>(raw).is_err());
        }

        assert!(seq::<u8>(json!([256])).is_err());
    }

    #[test]
    fn deserializes_optional_single_param_sequence() {
        for raw in [json!([]), json!([null]), json!(null)] {
            assert_eq!(opt_seq::<u64>(raw.clone()).unwrap(), None);
            assert_eq!(opt_seq::<U256>(raw).unwrap(), None);
        }

        for raw in [
            json!([1, 2]),
            json!([["0x64"]]),
            json!(["0x10000000000000000"]),
            json!(["18446744073709551616"]),
        ] {
            assert!(opt_seq::<u64>(raw).is_err());
        }
    }

    #[test]
    fn reports_the_actual_sequence_length() {
        let err = seq::<u64>(json!([1, 2, 3])).unwrap_err();
        assert!(err.to_string().starts_with("invalid length 3"), "{err}");

        let err = opt_seq::<u64>(json!([1, 2])).unwrap_err();
        assert!(err.to_string().starts_with("invalid length 2"), "{err}");
    }

    #[test]
    fn deserializes_params_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Params {
            #[serde(deserialize_with = "super::seq::deserialize")]
            block: u64,
            #[serde(default, deserialize_with = "super::seq::opt::deserialize")]
            timestamp: Option<U256>,
        }

        let params: Params =
            serde_json::from_str(r#"{"block": ["0x64"], "timestamp": ["1700000000"]}"#).unwrap();
        assert_eq!(params, Params { block: 100, timestamp: Some(U256::from(1700000000u64)) });

        let params: Params = serde_json::from_str(r#"{"block": 100, "timestamp": []}"#).unwrap();
        assert_eq!(params, Params { block: 100, timestamp: None });

        let params: Params = serde_json::from_str(r#"{"block": "100"}"#).unwrap();
        assert_eq!(params, Params { block: 100, timestamp: None });
    }
}
