//! Types for the `debug` API.

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use alloy_primitives::{Bytes, StorageKey, B256};
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

/// Wrapper type for a map of storage slots.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMap(pub BTreeMap<B256, StorageResult>);

/// Represents the result of a storage range query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRangeResult {
    /// A map of storage slots
    pub storage: StorageMap,
    /// The next key
    pub next_key: Option<B256>,
}

/// Represents the execution witness of a block. Contains an optional map of state preimages.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWitness {
    /// List of all hashed trie nodes preimages that were required during the execution of
    /// the block, including during state root recomputation.
    pub state: Vec<Bytes>,
    /// List of all contract codes (created / accessed) preimages that were required during
    /// the execution of the block, including during state root recomputation.
    pub codes: Vec<Bytes>,
    /// List of all hashed account and storage keys (addresses and slots) preimages
    /// (unhashed account addresses and storage slots, respectively) that were required during
    /// the execution of the block.
    pub keys: Vec<Bytes>,
    /// Block headers required for proving correctness of stateless execution.
    ///
    /// This collection stores ancestor(parent) block headers needed to verify:
    /// - State reads are correct (ie the code and accounts are correct wrt the pre-state root)
    /// - BLOCKHASH opcode execution results are correct
    ///
    /// ## Why this field will be empty in the future
    ///
    /// This field is expected to be empty in the future because:
    /// - EIP-2935 (Prague) will include block hashes directly in the state
    /// - Verkle/Delayed execution will change the block structure to contain the pre-state root
    ///   instead of the post-state root.
    ///
    /// Once both of these upgrades have been implemented, this field will be empty
    /// moving forward because the data that this was proving will either be in the
    /// current block or in the state.
    ///
    /// ## State Read Verification
    ///
    /// To verify state reads are correct, we need the pre-state root of the current block,
    /// which is (currently) equal to the post-state root of the previous block. We therefore
    /// need the previous block's header in order to prove that the state reads are correct.
    ///
    /// Note: While the pre-state root is located in the previous block, this field
    /// will always have one or more items.
    ///
    /// ## BLOCKHASH Opcode Verification
    ///
    /// The BLOCKHASH opcode returns the block hash for a given block number, but it
    /// only works for the 256 most recent blocks, not including the current block.
    /// To verify that a block hash is indeed correct wrt the BLOCKHASH opcode
    /// and not an arbitrary set of block hashes, we need a contiguous set of
    /// block headers starting from the current block.
    ///
    /// ### Example
    ///
    /// Consider a blockchain at block 200, and inside of block 200, a transaction
    /// calls BLOCKHASH(100):
    /// - This is valid because block 100 is within the 256-block lookback window
    /// - To verify this, we need all of the headers from block 100 through block 200
    /// - These headers form a chain proving the correctness of block 100's hash.
    ///
    /// The naive way to construct the headers would be to unconditionally include the last
    /// 256 block headers. However note, we may not need all 256, like in the example above.
    pub headers: Vec<Bytes>,
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
