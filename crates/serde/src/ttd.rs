//! Serde functions for encoding the TTD using a Geth compatible format.
//!
//! In Go `big.Int`s are marshalled as a JSON number without quotes. Numbers
//! are arbitrary precision in JSON, so this is valid JSON, but by default a
//! `U256` use hex encoding.
//! These functions encode the TTD as an `u128`, which is sufficient even for
//! the Ethereum mainnet TTD.
//!
//! This is only enabled for human-readable [`serde`] implementations.

use alloy_primitives::U256;
use serde::{ser, Deserialize, Deserializer, Serialize, Serializer};

/// Serializes an optional TTD as a JSON number.
///
/// It returns an error, if the TTD value is larger than 128-bit.
pub fn serialize<S>(value: &Option<U256>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        match value {
            Some(value) => {
                // convert into an u128 when possible
                let number = value.try_into().map_err(ser::Error::custom)?;
                serializer.serialize_u128(number)
            }
            None => serializer.serialize_none(),
        }
    } else {
        value.serialize(serializer)
    }
}

/// Deserializes an optional TTD value from a 128-bit JSON number.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        Ok(Option::<u128>::deserialize(deserializer)?.map(U256::from))
    } else {
        Option::<U256>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::{vec, vec::Vec};
    use alloy_primitives::U256;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[test]
    fn deserialize_ttd() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        let deserialized: Vec<Ttd> = serde_json::from_str(
            "[0, 17000000000000000, 58750000000000000000000, 4294967295, 18446744073709551615, 340282366920938463463374607431768211455]",
        )
        .unwrap();
        assert_eq!(
            deserialized,
            vec![
                Ttd(Some(U256::ZERO)),
                Ttd(Some(U256::from(17000000000000000u64))),
                Ttd(Some(U256::from(58750000000000000000000u128))),
                Ttd(Some(U256::from(4294967295u32))),
                Ttd(Some(U256::from(18446744073709551615u64))),
                Ttd(Some(U256::from(340282366920938463463374607431768211455u128))),
            ]
        );
    }

    #[test]
    fn serialize_ttd() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        let tests = vec![
            Ttd(Some(U256::ZERO)),
            Ttd(Some(U256::from(17000000000000000u64))),
            Ttd(Some(U256::from(58750000000000000000000u128))),
        ];

        for test in tests {
            let str = serde_json::to_string(&test).unwrap();
            // should be serialized as a decimal number and not a quoted string
            let num = u128::from_str_radix(&str, 10).unwrap();
            assert!(matches!(test, Ttd(Some(ttd)) if ttd == U256::from(num)));
        }
    }

    #[test]
    #[ignore = "serde_json does not handle untagged enums correctly: https://github.com/serde-rs/serde/issues/2230"]
    fn deserialize_ttd_untagged_enum() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        enum Ttd {
            TTD(#[serde(with = "super")] Option<U256>),
        }
        let test = Ttd::TTD(Some(U256::from(58750000000000000000000u128)));
        let str = serde_json::to_string(&test).unwrap();
        // should not be serialized as a quoted string
        assert!(str.ends_with("}") && !str.ends_with("\"}"));

        let deserialized: Ttd =
            serde_json::from_value(json!({"TTD": 58750000000000000000000u128})).unwrap();
        assert_eq!(deserialized, test);
    }

    #[test]
    fn deserialize_ttd_none() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // Deserialize null as None
        let deserialized: Ttd = serde_json::from_value(json!(null)).unwrap();
        assert_eq!(deserialized, Ttd(None));
    }

    #[test]
    fn deserialize_ttd_string() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // strings, even hex, are not allowed
        let result: Result<Ttd, _> = serde_json::from_value(json!("0x0"));
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_ttd_negative_number() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // Test for a negative number which should not be allowed
        let result: Result<Ttd, _> = serde_json::from_value(json!(-1));
        assert!(result.is_err());
    }
}
