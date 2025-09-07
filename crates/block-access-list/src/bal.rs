//! Contains the `BlockAccessList` struct, which represents a simple list of account changes.

use crate::account_change::AccountChanges;
use alloc::vec::Vec;

/// Vector of account changes.
pub type BlockAccessList = Vec<AccountChanges>;

#[cfg(test)]
mod tests {
    use alloy_primitives::keccak256;

    use crate::BlockAccessList;

    #[test]
    fn test_storage() {
        //     let key = U256::from(1);
        //     let bkey = StorageKey::from(U256::from(1));
        // let key_hash = keccak256(alloy_rlp::encode(key));
        // let bkey_hash = keccak256(alloy_rlp::encode(bkey));
        // println!("Key hash for {:?}: {:?}", key, key_hash);
        // println!("Bkey hash for {:?}: {:?}", bkey, bkey_hash);
        //         assert_ne!(key_hash, bkey_hash);
        //         let
        // bal_hash=keccak256("
        // 0xf901f1f89f9400000961ef480eb55e80d19ad83579a64c007002c0f884a00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000000000000000000000000003c0c0c0f89f940000bbddc7ce488642fb579f8b00f3a590007251c0f884a00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000000000000000000000000003c0c0c0e9940cc3e75918bf790b1a23a2d210a81db54e305930c0c0cccb01893635c9adc5de9c0824c3c20101c0e0942adc25665018aa1fe0e6bc666dac8fc2697ff9bac0c0c6c501830130c2c0c0f86294f480127a5d253e382993030c0afdc0751b0eb2fcf847f845a00000000000000000000000000000000000000000000000000000000000000001e3e201a00000000000000000000000000000000000000000000000000000000000000042c0c0c0c0"
        // );         println!("BAL hash: {:?}", bal_hash);
        let bal: BlockAccessList = serde_json::from_str(
            r#"[
  {
    "address": "0x00000961ef480eb55e80d19ad83579a64c007002",
    "storageChanges": [],
    "storageReads": [
      "0x0000000000000000000000000000000000000000000000000000000000000000",
      "0x0000000000000000000000000000000000000000000000000000000000000001",
      "0x0000000000000000000000000000000000000000000000000000000000000002",
      "0x0000000000000000000000000000000000000000000000000000000000000003"
    ],
    "balanceChanges": [],
    "nonceChanges": [],
    "codeChanges": []
  },
  {
    "address": "0x0000bbddc7ce488642fb579f8b00f3a590007251",
    "storageChanges": [],
    "storageReads": [
      "0x0000000000000000000000000000000000000000000000000000000000000000",
      "0x0000000000000000000000000000000000000000000000000000000000000001",
      "0x0000000000000000000000000000000000000000000000000000000000000002",
      "0x0000000000000000000000000000000000000000000000000000000000000003"
    ],
    "balanceChanges": [],
    "nonceChanges": [],
    "codeChanges": []
  },
  {
    "address": "0x0cc3e75918bf790b1a23a2d210a81db54e305930",
    "storageChanges": [],
    "storageReads": [],
    "balanceChanges": [
      {
        "txIndex": "1",
        "postBalance": "0x3635c9adc5de9c0824"
      }
    ],
    "nonceChanges": [
      {
        "txIndex": "1",
        "postNonce": "1"
      }
    ],
    "codeChanges": []
  },
  {
    "address": "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba",
    "storageChanges": [],
    "storageReads": [],
    "balanceChanges": [
      {
        "txIndex": "1",
        "postBalance": "0x130c2"
      }
    ],
    "nonceChanges": [],
    "codeChanges": []
  },
  {
    "address": "0xf480127a5d253e382993030c0afdc0751b0eb2fc",
    "storageChanges": [
      {
        "slot": "0x0000000000000000000000000000000000000000000000000000000000000001",
        "slotChanges": [
          {
            "txIndex": "1",
            "postValue":
"0x0000000000000000000000000000000000000000000000000000000000000042"           }
        ]
      }
    ],
    "storageReads": [],
    "balanceChanges": [],
    "nonceChanges": [],
    "codeChanges": []
  }
]

"#,
        )
        .unwrap();
        let hash = keccak256(alloy_rlp::encode(bal));
        println!("BAL hash: {:?}", hash);
        println!("Expected: 0xe682dafca027189a9b300765651aafb3ec4f02e04faaf027bbbe7b50ff2f43c9");
    }
}
