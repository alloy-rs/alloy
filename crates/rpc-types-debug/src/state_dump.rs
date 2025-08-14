use alloc::collections::btree_map::BTreeMap;
use alloy_primitives::{Address, Bytes, StorageKey, StorageValue, B256, U256};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

/// Represents the state of an account in the Ethereum state trie.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountState {
    /// The balance of the account
    pub balance: U256,
    /// The nonce of the account
    pub nonce: u64,
    /// The root hash of the account
    pub root: B256,
    /// The code hash of the account
    pub code_hash: B256,
    /// The code of the account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Bytes>,
    /// A map of storage slots, indexed by storage key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<BTreeMap<StorageKey, StorageValue>>,
    /// Address only present in iterative (line-by-line) mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    /// If we don't have address, we can output the key
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "key")]
    pub address_hash: Option<B256>,
}

/// Represents a state dump, which includes the root hash of the state trie,
/// Note: There are some differences in JSON serialization compared to geth's output, such as:
/// - The root field in geth doesn't have a 0x prefix, while here it does
/// - The balance field of accounts in geth is a decimal string, while here it's a hexadecimal
///   string
/// - The value field of storage in geth doesn't have a 0x prefix, while here it does
#[serde_as]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDump {
    /// The root hash of the state trie
    pub root: B256,
    /// A map of accounts, indexed by address
    pub accounts: BTreeMap<Address, AccountState>,
    /// Next can be set to represent that this dump is only partial, and Next
    /// is where an iterator should be positioned in order to continue the dump.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Base64>")]
    pub next: Option<Bytes>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_state_dump_serialization() {
        let json_input = json!({
            "root": "0x5a1f70040e967bef6a32ee65e7fa2c3ea580e277e42cf3e3daf60a677ef18127",
            "accounts": {
                "0x000baa01f2a21d29dce20b88032752b990dac124": {
                    "balance": "0x10000000000000000000",
                    "nonce": 0,
                    "root": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                    "codeHash": "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
                    "address": "0x000baa01f2a21d29dce20b88032752b990dac124",
                    "key": "0x000108a52c8b050f1098144f89e0b8e7e41310ea139f020b690b56e424508f4c"
                },
                "0x201d43c399f2495e19a591eab93fa3384ec6c72e": {
                    "balance": "0x0",
                    "nonce": 1,
                    "root": "0x297c068574a50ffef03843dda4075c3b6b5790be78b30e3c9df4e02e4ba9125c",
                    "codeHash": "0xbe6e2f7cdf118a0b2092927e0a0cf4a54316165ac5172bcda327939e04c9818f",
                    "code": "0x36602c57343d527f9e4ac34f21c619cefc926c8bd93b54bf5a39c7ab2127a895af1cc0691d7e3dff593da1005b363d3d373d3d3d3d610076806062363936013d732efa42b7d7591cbf436cce4973f900d8314c86dd5af43d3d93803e606057fd5bf34ad30ecfb92b9311a853d296c515fb0d6505d89c68db32372fd77e57b0879f97224bb89dac59e267486b38ee20309c8cc1acfb854eb9303a31c50a42f48a8fcc63b84d60abf8c5408ea569569af66c0cc3a76f6e00000000000000000000000000000000000000000000000000000000000af9ac0076",
                    "storage": {
                        "0x0000000000000000000000000000000000000000000000000000000000000000": "0x100000000000000000000000000686f559c",
                        "0x0000000000000000000000000000000000000000000000000000000000000002": "0x1",
                        "0x0000000000000000000000000000000000000000000000000000000000000008": "0xdead000000000000000000000000000000000000000000000000000000000000",
                        "0x000000000000000000000000000000000000000000000000000000000000000a": "0x1",
                        "0x405787fa12a823e0f2b7631cc41b3ba8828b3321ca811111fa75cd3aa3bb5ace": "0xffffffff",
                        "0x405787fa12a823e0f2b7631cc41b3ba8828b3321ca811111fa75cd3aa3bb5acf": "0x4ad30ecfb92b9311a853d296c515fb0d6505d89c",
                        "0x405787fa12a823e0f2b7631cc41b3ba8828b3321ca811111fa75cd3aa3bb5ad1": "0x68db32372fd77e57b0879f97224bb89dac59e267486b38ee20309c8cc1acfb85",
                        "0x405787fa12a823e0f2b7631cc41b3ba8828b3321ca811111fa75cd3aa3bb5ad2": "0x686f559c00000000000000000000000000000001"
                    },
                    "address": "0x201d43c399f2495e19a591eab93fa3384ec6c72e",
                    "key": "0x0000e65fdfaa2681656a211a55bc6fdcfe918f34cc037407ba12874c16cd7da9"
                }
            },
            "next": "AAEx7TCXUlkysLMMJcS/W974Ue7bbhgSK3EUHVNFCtQ="
        });

        let parsed: StateDump = serde_json::from_value(json_input.clone()).unwrap();
        let output = serde_json::to_value(&parsed).unwrap();
        assert_eq!(json_input, output);
    }
}
