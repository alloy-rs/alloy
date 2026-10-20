use alloc::collections::BTreeMap;
use alloy_primitives::{
    ruint::{BaseConvertError, ParseError},
    Bytes, B256, U256,
};
use core::{fmt, str::FromStr};
use serde::{de, Deserialize, Deserializer, Serialize};

/// A storage key that accepts hex strings up to 32 bytes for `eth_getStorageAt` and `eth_getProof`.
///
/// An input of 63 or 64 hex digits is stored as [`Hash`](Self::Hash) and serializes at full width.
/// Shorter inputs are stored as [`Number`](Self::Number) and serialize as canonical quantities, so
/// their original prefix, letter case, and leading zeroes are not preserved. [`Self::as_b256`]
/// returns either representation left-padded to 32 bytes.
///
/// In human-readable formats, a string key must be `0x`-prefixed hex with at most 64 digits. A key
/// with 63 or 64 digits is a [`Hash`](Self::Hash); a shorter key, including `0x`, is a
/// [`Number`](Self::Number). A bare 64-digit hex string is also accepted as a
/// [`Hash`](Self::Hash). Integer input is a [`Number`](Self::Number).
///
/// This distinction mirrors geth's treatment of `eth_getProof` storage keys. See how input
/// `storageKey`s populate the returned `StorageResult`:
/// <https://github.com/ethereum/go-ethereum/blob/00a73fbcce3250b87fc4160f3deddc44390848f4/internal/ethapi/api.go#L658-L690>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum JsonStorageKey {
    /// A key of 63 or 64 hex digits
    Hash(B256),
    /// A key of at most 62 hex digits, or an integer
    Number(U256),
}

impl<'de> Deserialize<'de> for JsonStorageKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(JsonStorageKeyVisitor)
        } else {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Untagged {
                Hash(B256),
                Number(U256),
            }
            Ok(match Untagged::deserialize(deserializer)? {
                Untagged::Hash(hash) => Self::Hash(hash),
                Untagged::Number(num) => Self::Number(num),
            })
        }
    }
}

struct JsonStorageKeyVisitor;

impl de::Visitor<'_> for JsonStorageKeyVisitor {
    type Value = JsonStorageKey;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a 0x-prefixed hex string of at most 64 digits, or an integer")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(JsonStorageKey::Number(U256::from(v)))
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
        Ok(JsonStorageKey::Number(U256::from(v)))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        // The derived parser accepted a bare 64-digit key, so keep it.
        let digits = match v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
            Some(digits) => digits,
            None if v.len() == 64 => v,
            None => return Err(E::invalid_value(de::Unexpected::Str(v), &self)),
        };
        if digits.len() > 64 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(E::invalid_value(de::Unexpected::Str(v), &self));
        }
        let num = if digits.is_empty() {
            U256::ZERO
        } else {
            U256::from_str_radix(digits, 16).map_err(E::custom)?
        };
        Ok(if digits.len() >= 63 { JsonStorageKey::Hash(num.into()) } else { num.into() })
    }
}

impl JsonStorageKey {
    /// Returns the key as a [`B256`] value.
    pub fn as_b256(&self) -> B256 {
        match self {
            Self::Hash(hash) => *hash,
            Self::Number(num) => B256::from(*num),
        }
    }
}

impl Default for JsonStorageKey {
    fn default() -> Self {
        Self::Hash(Default::default())
    }
}

impl From<B256> for JsonStorageKey {
    fn from(value: B256) -> Self {
        Self::Hash(value)
    }
}

impl From<[u8; 32]> for JsonStorageKey {
    fn from(value: [u8; 32]) -> Self {
        B256::from(value).into()
    }
}

impl From<U256> for JsonStorageKey {
    fn from(value: U256) -> Self {
        Self::Number(value)
    }
}

impl FromStr for JsonStorageKey {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 65 && !(s.len() == 66 && s.starts_with("0x")) {
            return Err(ParseError::BaseConvertError(BaseConvertError::Overflow));
        }

        if let Ok(hash) = B256::from_str(s) {
            return Ok(Self::Hash(hash));
        }
        s.parse().map(Self::Number)
    }
}

impl fmt::Display for JsonStorageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(hash) => hash.fmt(f),
            Self::Number(num) => write!(f, "{num:#x}"),
        }
    }
}

/// Converts a [`Bytes`] value of at most 32 bytes into a [`B256`], left-padding it with zeroes.
pub fn from_bytes_to_b256<'de, D>(bytes: Bytes) -> Result<B256, D::Error>
where
    D: Deserializer<'de>,
{
    if bytes.0.len() > 32 {
        return Err(serde::de::Error::custom("input too long to be a B256"));
    }

    Ok(B256::left_padding_from(&bytes.0))
}

/// Deserializes an optional storage map, accepting keys and values shorter than 32 bytes and
/// left-padding them with zeroes:
///
/// ```json
/// {
///     "0x0000000000000000000000000000000000000000000000000000000000000001": "0x22"
/// }
/// ```
pub fn deserialize_storage_map<'de, D>(
    deserializer: D,
) -> Result<Option<BTreeMap<B256, B256>>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        let map = Option::<BTreeMap<Bytes, Bytes>>::deserialize(deserializer)?;
        match map {
            Some(map) => {
                let mut res_map = BTreeMap::new();
                for (k, v) in map {
                    let k_deserialized = from_bytes_to_b256::<'de, D>(k)?;
                    let v_deserialized = from_bytes_to_b256::<'de, D>(v)?;
                    res_map.insert(k_deserialized, v_deserialized);
                }
                Ok(Some(res_map))
            }
            None => Ok(None),
        }
    } else {
        Option::<BTreeMap<B256, B256>>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use serde_json::json;

    #[test]
    fn test_storage_key() {
        let cases = [
            "0x0000000000000000000000000000000000000000000000000000000000000001", // Hash
            "0000000000000000000000000000000000000000000000000000000000000001",   // Hash
        ];

        let key: JsonStorageKey = serde_json::from_str(&json!(cases[0]).to_string()).unwrap();
        let key2: JsonStorageKey = serde_json::from_str(&json!(cases[1]).to_string()).unwrap();

        assert_eq!(key.as_b256(), key2.as_b256());
    }

    #[test]
    fn test_storage_key_serde_roundtrips() {
        let test_cases = [
            "0x0000000000000000000000000000000000000000000000000000000000000001", // Hash
            "0x0000000000000000000000000000000000000000000000000000000000000abc", // Hash
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",   // Number
            "0xabc",                                                              // Number
            "0xabcd",                                                             // Number
            // 0x + 64 hex chars (max U256, total len = 66)
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", // Hash
        ];

        for input in test_cases {
            let key: JsonStorageKey = serde_json::from_str(&json!(input).to_string()).unwrap();
            let output = key.to_string();

            assert_eq!(
                input, output,
                "Storage key roundtrip failed to preserve the exact hex representation for {input}"
            );
        }
    }

    #[test]
    fn test_as_b256() {
        let cases = [
            "0x0abc",                                                             // Number
            "0x0000000000000000000000000000000000000000000000000000000000000abc", // Hash
        ];

        let num_key: JsonStorageKey = serde_json::from_str(&json!(cases[0]).to_string()).unwrap();
        let hash_key: JsonStorageKey = serde_json::from_str(&json!(cases[1]).to_string()).unwrap();

        assert_eq!(num_key, JsonStorageKey::Number(U256::from_str(cases[0]).unwrap()));
        assert_eq!(hash_key, JsonStorageKey::Hash(B256::from_str(cases[1]).unwrap()));

        assert_eq!(num_key.as_b256(), hash_key.as_b256());
    }

    #[test]
    fn test_deserialize_storage_key_widths() {
        let parse = |s: &str| serde_json::from_str::<JsonStorageKey>(&json!(s).to_string());

        let key = parse(&("0x".to_string() + &"f".repeat(64))).unwrap();
        assert_eq!(key, JsonStorageKey::Hash(B256::repeat_byte(0xff)));

        let key = parse(&("0x".to_string() + &"f".repeat(63))).unwrap();
        assert_eq!(key, JsonStorageKey::Hash(B256::from(U256::MAX >> 4)));
        assert_eq!(key.to_string(), "0x0".to_string() + &"f".repeat(63));

        let key = parse(&("0x".to_string() + &"f".repeat(62))).unwrap();
        assert_eq!(key, JsonStorageKey::Number(U256::MAX >> 8));

        assert_eq!(parse("0x").unwrap(), JsonStorageKey::Number(U256::ZERO));
        assert_eq!(parse("0x").unwrap().to_string(), "0x0");
        assert_eq!(parse("0X0aBc").unwrap(), JsonStorageKey::Number(U256::from(0xabc)));
        assert_eq!(parse(&"0".repeat(64)).unwrap(), JsonStorageKey::Hash(B256::ZERO));
    }

    #[test]
    fn test_deserialize_invalid_storage_key() {
        let cases = [
            "0x".to_string() + &"0".repeat(65),
            "0x".to_string() + &"0".repeat(66),
            "0xzz".to_string(),
            "0x_1".to_string(),
            "0x 1".to_string(),
            "123".to_string(),
            String::new(),
            "f".repeat(63),
        ];
        for case in cases {
            let result = serde_json::from_str::<JsonStorageKey>(&json!(case).to_string());
            assert!(result.is_err(), "{case:?} should fail deserialization");
        }
    }

    #[test]
    fn test_deserialize_storage_key_integer() {
        let key: JsonStorageKey = serde_json::from_str("123").unwrap();
        assert_eq!(key, JsonStorageKey::Number(U256::from(123)));
    }

    #[test]
    fn test_deserialize_too_long_storage_key() {
        let key = "0x".to_string() + &"f".repeat(68);
        let result: Result<JsonStorageKey, _> = serde_json::from_str(&json!(key).to_string());
        assert!(result.is_err(), "storage key with 68 hex chars should fail deserialization");
    }

    #[test]
    fn test_from_str_length_boundaries() {
        // 0x + 63 hex chars = 65 total — valid U256
        let key_63 = "0x".to_string() + &"f".repeat(63);
        let result = JsonStorageKey::from_str(&key_63);
        assert!(result.is_ok(), "0x + 63 hex chars should be a valid U256 storage key");
        assert!(matches!(result.unwrap(), JsonStorageKey::Number(_)));

        // 0x + 64 hex chars = 66 total — valid B256
        let key_64 = "0x".to_string() + &"f".repeat(64);
        let result = JsonStorageKey::from_str(&key_64);
        assert!(result.is_ok(), "0x + 64 hex chars should be a valid B256 storage key");
        assert!(matches!(result.unwrap(), JsonStorageKey::Hash(_)));

        // 0x + 65 hex chars = 67 total — overflow
        let key_65 = "0x".to_string() + &"f".repeat(65);
        assert!(JsonStorageKey::from_str(&key_65).is_err());

        // 64 bare hex chars — valid B256
        let bare_64 = "f".repeat(64);
        let result = JsonStorageKey::from_str(&bare_64);
        assert!(result.is_ok(), "64 bare hex chars should be a valid B256 storage key");
        assert!(matches!(result.unwrap(), JsonStorageKey::Hash(_)));

        // 65 bare hex chars — overflow
        let bare_65 = "f".repeat(65);
        assert!(JsonStorageKey::from_str(&bare_65).is_err());
    }

    #[test]
    fn test_deserialize_storage_map_with_valid_data() {
        let json_data = json!({
            "0x0000000000000000000000000000000000000000000000000000000000000001": "0x22",
            "0x0000000000000000000000000000000000000000000000000000000000000002": "0x33"
        });

        // Specify the deserialization type explicitly
        let deserialized: Option<BTreeMap<B256, B256>> = deserialize_storage_map(
            &serde_json::from_value::<serde_json::Value>(json_data).unwrap(),
        )
        .unwrap();

        assert_eq!(
            deserialized.unwrap(),
            BTreeMap::from([
                (B256::from(U256::from(1u128)), B256::from(U256::from(0x22u128))),
                (B256::from(U256::from(2u128)), B256::from(U256::from(0x33u128)))
            ])
        );
    }

    #[test]
    fn test_deserialize_storage_map_with_empty_data() {
        let json_data = json!({});
        let deserialized: Option<BTreeMap<B256, B256>> = deserialize_storage_map(
            &serde_json::from_value::<serde_json::Value>(json_data).unwrap(),
        )
        .unwrap();
        assert!(deserialized.unwrap().is_empty());
    }

    #[test]
    fn test_deserialize_storage_map_with_none() {
        let json_data = json!(null);
        let deserialized: Option<BTreeMap<B256, B256>> = deserialize_storage_map(
            &serde_json::from_value::<serde_json::Value>(json_data).unwrap(),
        )
        .unwrap();
        assert!(deserialized.is_none());
    }

    #[test]
    fn test_from_bytes_to_b256_with_valid_input() {
        // Test case with input less than 32 bytes, should be left-padded with zeros
        let bytes = Bytes::from(vec![0x1, 0x2, 0x3, 0x4]);
        let result = from_bytes_to_b256::<serde_json::Value>(bytes).unwrap();
        let expected = B256::from_slice(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            2, 3, 4,
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_from_bytes_to_b256_with_exact_32_bytes() {
        // Test case with input exactly 32 bytes long
        let bytes = Bytes::from(vec![
            0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            0x20,
        ]);
        let result = from_bytes_to_b256::<serde_json::Value>(bytes).unwrap();
        let expected = B256::from_slice(&[
            0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            0x20,
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_from_bytes_to_b256_with_input_too_long() {
        // Test case with input longer than 32 bytes, should return an error
        let bytes = Bytes::from(vec![0x1; 33]); // 33 bytes long
        let result = from_bytes_to_b256::<serde_json::Value>(bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "input too long to be a B256");
    }

    #[test]
    fn test_from_bytes_to_b256_with_empty_input() {
        // Test case with empty input, should be all zeros
        let bytes = Bytes::from(vec![]);
        let result = from_bytes_to_b256::<serde_json::Value>(bytes).unwrap();
        let expected = B256::from_slice(&[0; 32]); // All zeros
        assert_eq!(result, expected);
    }
}
