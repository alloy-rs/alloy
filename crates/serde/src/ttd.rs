//! Serde functions for encoding the TTD using a Geth compatible format.
//!
//! In Go `big.Int` is marshalled as a JSON number without quotes. Numbers are arbitrary
//! precision in JSON, so this is valid JSON.
//!
//! These functions serialize the TTD as a JSON number, if the value fits within `u128`.
//!
//! The TTD is parsed from:
//!   - JSON numbers: direct `u64` values, or the specific Ethereum mainnet TTD (e.g., `5.875e22` if
//!     represented as a float). Other floats or negative numbers will error.
//!   - JSON strings: these are parsed as `U256` (allowing hex or decimal strings).
//!
//! For non-human-readable formats, the default `serde` behavior for `Option<U256>` is used.

use alloy_primitives::U256;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

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
                // serialize as an u128 when possible, otherwise as a hex string
                match value.try_into() {
                    Ok(value) => serializer.serialize_u128(value),
                    Err(_) => value.serialize(serializer),
                }
            }
            None => serializer.serialize_none(),
        }
    } else {
        value.serialize(serializer)
    }
}

/// Deserializes an optional TTD value from JSON number or string.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        Option::<Value>::deserialize(deserializer)?
            .map_or_else(|| Ok(None), |value| ttd_from_value::<D>(value).map(Some))
    } else {
        Option::<U256>::deserialize(deserializer)
    }
}

/// Supports parsing the TTD as an `Option<u64>`, or `Option<f64>` specifically for the mainnet TTD
/// (5.875e22).
pub use deserialize as deserialize_json_ttd_opt;

/// Converts the given [serde_json::Value] into a `U256` value for TTD deserialization.
fn ttd_from_value<'de, D>(val: Value) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let val = match val {
        Value::Number(num) => num,
        Value::String(raw) => return raw.parse().map_err(<D::Error as de::Error>::custom),
        _ => return Err(de::Error::custom("TTD must be a number or string")),
    };

    let num = if let Some(val) = val.as_u64() {
        U256::from(val)
    } else if let Some(value) = val.as_f64() {
        // The ethereum mainnet TTD is 58750000000000000000000, and geth serializes this
        // without quotes, because that is how golang `big.Int`s marshal in JSON. Numbers
        // are arbitrary precision in JSON, so this is valid JSON. This number is also
        // greater than a `u64`.
        //
        // Unfortunately, serde_json only supports parsing up to `u64`, resorting to `f64`
        // once `u64` overflows:
        // <https://github.com/serde-rs/json/blob/4bc1eaa03a6160593575bc9bc60c94dba4cab1e3/src/de.rs#L1411-L1415>
        // <https://github.com/serde-rs/json/blob/4bc1eaa03a6160593575bc9bc60c94dba4cab1e3/src/de.rs#L479-L484>
        // <https://github.com/serde-rs/json/blob/4bc1eaa03a6160593575bc9bc60c94dba4cab1e3/src/de.rs#L102-L108>
        //
        // serde_json does have an arbitrary precision feature, but this breaks untagged
        // enums in serde:
        // <https://github.com/serde-rs/serde/issues/2230>
        // <https://github.com/serde-rs/serde/issues/1183>
        //
        // To solve this, we use the captured float and return the TTD as a U256 if it's equal.
        if value == 5.875e22 {
            U256::from(58750000000000000000000u128)
        } else {
            // We could try to convert to a u128 here but there would probably be loss of
            // precision, so we just return an error.
            return Err(de::Error::custom(
                "Deserializing a large non-mainnet TTD is not supported",
            ));
        }
    } else {
        // must be i64 - negative numbers are not supported
        return Err(de::Error::custom(
            "Negative TTD values are invalid and will not be deserialized",
        ));
    };

    Ok(num)
}
#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::{vec, vec::Vec};
    use alloy_primitives::U256;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[test]
    fn jsonu256_deserialize() {
        let deserialized: Vec<U256> =
            serde_json::from_str(r#"["","0", "0x","10",10,"0x10"]"#).unwrap();
        assert_eq!(
            deserialized,
            vec![
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                U256::from(10),
                U256::from(10),
                U256::from(16),
            ]
        );
    }

    #[test]
    fn jsonu256_serialize() {
        let data = U256::from(16);
        let serialized = serde_json::to_string(&data).unwrap();

        assert_eq!(serialized, r#""0x10""#);
    }

    #[test]
    fn deserialize_ttd() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        let deserialized: Vec<Ttd> = serde_json::from_str(
            r#"["",0,"0","0x0",18446744073709551615,58750000000000000000000,"58750000000000000000000","0xC70D808A128D7380000"]"#,
        )
        .unwrap();
        assert_eq!(
            deserialized,
            vec![
                Ttd(Some(U256::ZERO)),
                Ttd(Some(U256::ZERO)),
                Ttd(Some(U256::ZERO)),
                Ttd(Some(U256::ZERO)),
                Ttd(Some(U256::from(18446744073709551615u64))),
                Ttd(Some(U256::from(58750000000000000000000u128))),
                Ttd(Some(U256::from(58750000000000000000000u128))),
                Ttd(Some(U256::from(58750000000000000000000u128))),
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
            Ttd(Some(U256::from(u128::MAX))),
        ];

        for test in tests {
            let str = serde_json::to_string(&test).unwrap();
            // should be serialized as a decimal number and not a quoted string
            let num = str.parse::<u128>().unwrap();
            assert!(matches!(test, Ttd(Some(ttd)) if ttd == U256::from(num)));
        }
    }

    #[test]
    fn deserialize_ttd_untagged_enum() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(untagged)]
        enum Ttd {
            Ttd(#[serde(with = "super")] Option<U256>),
        }
        let test = Ttd::Ttd(Some(U256::from(58750000000000000000000u128)));
        let str = serde_json::to_string(&test).unwrap();
        // should be serialized as an integer, not a float or a quoted string
        assert_eq!(str, r#"58750000000000000000000"#);

        let deserialized: Ttd = serde_json::from_str(&str).unwrap();
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
    fn deserialize_ttd_invalid_string() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // Invalid string that cannot be parsed into U256
        let result: Result<Ttd, _> = serde_json::from_value(json!("invalid_string"));
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_ttd_large_non_mainnet() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // Test for a large number not equal to 5.875e22, which should result in an error
        let result: Result<Ttd, _> = serde_json::from_value(json!(6.0e22));
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

    #[test]
    fn deserialize_ttd_as_string() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Ttd(#[serde(with = "super")] Option<U256>);

        // Test for valid TTD as a string
        let deserialized: Ttd = serde_json::from_value(json!("0x12345")).unwrap();
        assert_eq!(deserialized, Ttd(Some(U256::from(0x12345))));
    }
}
