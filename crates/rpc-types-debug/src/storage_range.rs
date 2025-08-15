//! Types for the `debug` API.

use alloc::collections::btree_map::BTreeMap;
use alloy_primitives::{StorageKey, B256};
use derive_more::{AsMut, AsRef, Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Represents the result of a storage slot query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageResult {
    /// The storage key
    pub key: StorageKey,
    /// The value stored at the slot
    pub value: B256,
}

impl From<(StorageKey, B256)> for StorageResult {
    fn from((key, value): (StorageKey, B256)) -> Self {
        Self { key, value }
    }
}

impl From<StorageResult> for (StorageKey, B256) {
    fn from(result: StorageResult) -> Self {
        (result.key, result.value)
    }
}

/// Wrapper type for a map of storage slots.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, AsRef, Deref, AsMut, DerefMut,
)]
#[serde(rename_all = "camelCase")]
pub struct StorageMap(pub BTreeMap<B256, StorageResult>);

impl From<BTreeMap<B256, StorageResult>> for StorageMap {
    fn from(map: BTreeMap<B256, StorageResult>) -> Self {
        Self(map)
    }
}

impl From<StorageMap> for BTreeMap<B256, StorageResult> {
    fn from(map: StorageMap) -> Self {
        map.0
    }
}

/// Represents the result of a storage range query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRangeResult {
    /// A map of storage slots
    pub storage: StorageMap,
    /// The next key
    pub next_key: Option<B256>,
}

impl From<(StorageMap, Option<B256>)> for StorageRangeResult {
    fn from((storage, next_key): (StorageMap, Option<B256>)) -> Self {
        Self { storage, next_key }
    }
}

impl From<StorageRangeResult> for (StorageMap, Option<B256>) {
    fn from(result: StorageRangeResult) -> Self {
        (result.storage, result.next_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_storage_range_result_roundtrip() {
        let json_input = json!({
          "storage": {
            "0x0000000000000000000000000000000000000000000000000000000000000002": {
              "key": "0x0000000000000000000000000000000000000000000000000000000000000002",
              "value": "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
            },
            "0x0000000000000000000000000000000000000000000000000000000000000003": {
              "key": "0x0000000000000000000000000000000000000000000000000000000000000003",
              "value": "0x0000000000000000000000000000000000000000000000000000000000000006"
            }
          },
          "nextKey": "0x0000000000000000000000000000000000000000000000000000000000000004"
        });

        let parsed: StorageRangeResult = serde_json::from_value(json_input.clone()).unwrap();

        let output = serde_json::to_value(&parsed).unwrap();

        assert_eq!(json_input, output);
    }
}
