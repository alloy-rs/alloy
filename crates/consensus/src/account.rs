use alloy_primitives::{B256, U256};

/// Represents an Account in the account trie.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Account {
    /// The account's balance.
    pub balance: U256,
    /// The hash of the code of the account.
    pub code_hash: B256,
    /// The account's nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub nonce: u64,
    /// The hash of the storage account data.
    pub storage_root: B256,
}
