use crate::constants::{EMPTY_ROOT_HASH, KECCAK_EMPTY};
use alloy_primitives::{keccak256, B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};

#[cfg(feature = "std")]
use {crate::proofs::storage_root_unhashed, alloy_genesis::GenesisAccount};

/// Represents an Account in the account trie.
#[derive(Copy, Clone, Debug, PartialEq, Eq, RlpDecodable, RlpEncodable)]
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

impl Default for Account {
    fn default() -> Self {
        Self {
            nonce: 0,
            balance: U256::ZERO,
            storage_root: EMPTY_ROOT_HASH,
            code_hash: KECCAK_EMPTY,
        }
    }
}

impl Account {
    /// Compute  hash as committed to in the MPT trie without memorizing.
    pub fn trie_hash_slow(&self) -> B256 {
        keccak256(alloy_rlp::encode(self))
    }
}

#[cfg(feature = "std")]
impl From<GenesisAccount> for Account {
    fn from(account: GenesisAccount) -> Self {
        let storage_root = account
            .storage
            .map(|storage| {
                storage_root_unhashed(
                    storage
                        .into_iter()
                        .filter(|(_, value)| !value.is_zero())
                        .map(|(slot, value)| (slot, U256::from_be_bytes(*value))),
                )
            })
            .unwrap_or(EMPTY_ROOT_HASH);

        Self {
            nonce: account.nonce.unwrap_or_default(),
            balance: account.balance,
            storage_root,
            code_hash: account.code.map_or(KECCAK_EMPTY, keccak256),
        }
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

        let encoded = alloy_rlp::encode(account);

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

        let expected_hash = keccak256(alloy_rlp::encode(account));
        let actual_hash = account.trie_hash_slow();
        assert_eq!(expected_hash, actual_hash);
    }
}
