use alloy_primitives::{keccak256, B256, U256};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
/// Represents an Account in the account trie.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, RlpDecodable, RlpEncodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Account {
    /// The account's nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// The account's balance.
    pub balance: U256,
    /// The hash of the storage account data.
    pub storage_root: B256,
    /// The hash of the code of the account.
    pub code_hash: B256,
}

impl Account {
    /// Compute  hash as committed to in the MPT trie without memoizing.
    pub fn trie_hash_slow(&self) -> B256 {
        let mut buf = vec![];
        self.encode(&mut buf);
        keccak256(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{hex, U256};
    use alloy_rlp::Decodable;

    #[test]
    fn test_account_encoding() {
        let account = Account {
            nonce: 1,
            balance: U256::from(1000),
            storage_root: B256::from_slice(&hex!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            )),
            code_hash: keccak256(hex!("5a465a905090036002900360015500")),
        };

        let mut encoded = vec![];
        account.encode(&mut encoded);

        let decoded = Account::decode(&mut &encoded[..]).unwrap();
        assert_eq!(account, decoded);
    }

    #[test]
    fn test_trie_hash_slow() {
        let account = Account {
            nonce: 1,
            balance: U256::from(1000),
            storage_root: B256::from_slice(&hex!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            )),
            code_hash: keccak256(hex!("5a465a905090036002900360015500")),
        };

        let expected_hash = keccak256(account.encode_to_vec());
        let actual_hash = account.trie_hash_slow();
        assert_eq!(expected_hash, actual_hash);
    }
    impl Account {
        fn encode_to_vec(&self) -> Vec<u8> {
            let mut out = vec![];
            self.encode(&mut out);
            out
        }
    }
}
