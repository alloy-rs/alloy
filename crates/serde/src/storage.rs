use core::str::FromStr;

use alloc::{
    collections::BTreeMap,
    fmt::Write,
    string::{String, ToString},
};
use alloy_primitives::{Bytes, B256, U256};
use serde::{Deserialize, Deserializer, Serialize};

/// A storage key kind that can be either [B256] or [U256].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageKeyKind {
    /// A full 32-byte key (tried first during deserialization)
    Hash(B256),
    /// A number (fallback if B256 deserialization fails)
    Number(U256),
}

impl Default for StorageKeyKind {
    fn default() -> Self {
        Self::Number(U256::ZERO)
    }
}

/// A storage key type that can be serialized to and from a hex string up to 32 bytes. Used for
/// `eth_getStorageAt` and `eth_getProof` RPCs.
///
/// This is a wrapper type meant to mirror geth's serialization and deserialization behavior for
/// storage keys.
///
/// In `eth_getStorageAt`, this is used for deserialization of the `index` field. Internally, the
/// index is a [B256], but in `eth_getStorageAt` requests, its serialization can be _up to_ 32
/// bytes. To support this, the storage key is deserialized first as a U256, and converted to a
/// B256 for use internally.
///
/// `eth_getProof` also takes storage keys up to 32 bytes as input, so the `keys` field is
/// similarly deserialized. However, geth populates the storage proof `key` fields in the response
/// by mirroring the `key` field used in the input.
///
/// See how `storageKey`s (the input) are populated in the `StorageResult` (the output):
/// <https://github.com/ethereum/go-ethereum/blob/00a73fbcce3250b87fc4160f3deddc44390848f4/internal/ethapi/api.go#L658-L690>
///
/// The contained [B256] and From implementation for String are used to preserve the input and
/// implement this behavior from geth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct JsonStorageKey(pub StorageKeyKind);

impl JsonStorageKey {
    /// Returns the key as a [B256] value.
    pub fn as_b256(&self) -> B256 {
        match self.0 {
            StorageKeyKind::Hash(hash) => hash,
            StorageKeyKind::Number(num) => B256::from(num),
        }
    }
}

impl<'de> Deserialize<'de> for JsonStorageKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Try B256 first
        if let Ok(hash) = B256::from_str(&s) {
            return Ok(Self(StorageKeyKind::Hash(hash)));
        }

        // Fallback to U256
        let number = U256::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self(StorageKeyKind::Number(number)))
    }
}

impl From<B256> for JsonStorageKey {
    fn from(value: B256) -> Self {
        Self(StorageKeyKind::Hash(value))
    }
}

impl From<[u8; 32]> for JsonStorageKey {
    fn from(value: [u8; 32]) -> Self {
        B256::from(value).into()
    }
}

impl From<U256> for JsonStorageKey {
    fn from(value: U256) -> Self {
        // SAFETY: Address (B256) and U256 have the same number of bytes
        value.to_be_bytes().into()
    }
}

impl From<JsonStorageKey> for String {
    fn from(value: JsonStorageKey) -> Self {
        match value.0 {
            // SAFETY: Address (B256) and U256 have the same number of bytes
            // serialize byte by byte
            StorageKeyKind::Hash(hash) => {
                // For Hash variant, preserve the full 32-byte representation
                let mut hex = Self::with_capacity(66); // 2 + 64
                hex.push_str("0x");
                for byte in hash.as_slice() {
                    write!(hex, "{:02x}", byte).unwrap();
                }
                hex
            }
            StorageKeyKind::Number(num) => {
                // this is mainly so we can return an output that hive testing expects, because the
                // `eth_getProof` implementation in geth simply mirrors the input
                //
                // see the use of `hexKey` in the `eth_getProof` response:
                // <https://github.com/ethereum/go-ethereum/blob/b87b9b45331f87fb1da379c5f17a81ebc3738c6e/internal/ethapi/api.go#L689-L763>
                // For Number variant, use the trimmed representation
                let bytes = num.to_be_bytes_trimmed_vec();
                // Early return if the input is empty. This case is added to satisfy the hive tests.
                // <https://github.com/ethereum/go-ethereum/blob/b87b9b45331f87fb1da379c5f17a81ebc3738c6e/internal/ethapi/api.go#L727-L729>
                if bytes.is_empty() {
                    return "0x0".to_string();
                }
                let mut hex = Self::with_capacity(2 + bytes.len() * 2);
                hex.push_str("0x");
                for byte in bytes {
                    write!(hex, "{:02x}", byte).unwrap();
                }
                hex
            }
        }
    }
}

/// Converts a Bytes value into a B256, accepting inputs that are less than 32 bytes long. These
/// inputs will be left padded with zeros.
pub fn from_bytes_to_b256<'de, D>(bytes: Bytes) -> Result<B256, D::Error>
where
    D: Deserializer<'de>,
{
    if bytes.0.len() > 32 {
        return Err(serde::de::Error::custom("input too long to be a B256"));
    }

    // left pad with zeros to 32 bytes
    let mut padded = [0u8; 32];
    padded[32 - bytes.0.len()..].copy_from_slice(&bytes.0);

    // then convert to B256 without a panic
    Ok(B256::from_slice(&padded))
}

/// Deserializes the input into a storage map, using [from_bytes_to_b256] which allows cropped
/// values:
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_storage_key() {
        let key = JsonStorageKey::default();
        assert_eq!(String::from(key), String::from("0x0"));
    }

    #[test]
    fn test_storage_key() {
        let cases = [
            "0x0000000000000000000000000000000000000000000000000000000000000001", // Hash
            "0000000000000000000000000000000000000000000000000000000000000001",   // Hash
        ];

        let key: JsonStorageKey = serde_json::from_str(&json!(cases[0]).to_string()).unwrap();
        let key2: JsonStorageKey = serde_json::from_str(&json!(cases[1]).to_string()).unwrap();

        assert_eq!(key, key2);

        let output = String::from(key);
        let output2 = String::from(key2);

        assert_eq!(output, output2);
    }

    #[test]
    fn test_storage_key_serde_roundtrips() {
        let test_cases = [
            "0x0000000000000000000000000000000000000000000000000000000000000001", // Hash
            "0x0000000000000000000000000000000000000000000000000000000000000abc", // Hash
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",   // Number
            "0x0abc",                                                             // Number
            "0xabcd",                                                             // Number
        ];

        for input in test_cases {
            let key: JsonStorageKey = serde_json::from_str(&json!(input).to_string()).unwrap();
            let output = String::from(key);

            assert_eq!(
                input, output,
                "Storage key roundtrip failed to preserve the exact hex representation for {}",
                input
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

        assert_eq!(num_key.0, StorageKeyKind::Number(U256::from_str(cases[0]).unwrap()));
        assert_eq!(hash_key.0, StorageKeyKind::Hash(B256::from_str(cases[1]).unwrap()));

        assert_eq!(num_key.as_b256(), hash_key.as_b256());
    }
}
