//! Helper function for calculating Merkle proofs and hashes.

use crate::{Account, Header, EMPTY_OMMER_ROOT_HASH};
use alloc::vec::Vec;
use alloy_eips::{eip2718::Encodable2718, eip4895::Withdrawal};
use alloy_primitives::{keccak256, Address, B256, U256};
use alloy_trie::{
    root::{ordered_trie_root, ordered_trie_root_with_encoder},
    HashBuilder, Nibbles,
};
use itertools::Itertools;

/// Calculate a transaction root.
///
/// `(rlp(index), encoded(tx))` pairs.
pub fn calculate_transaction_root<T, E>(transactions: &[T]) -> B256
where
    T: Encodable2718,
{
    ordered_trie_root_with_encoder(transactions, |tx: &T, buf| tx.encode_2718(buf))
}

/// Calculates the root hash of the withdrawals.
pub fn calculate_withdrawals_root(withdrawals: &[Withdrawal]) -> B256 {
    ordered_trie_root(withdrawals)
}

/// Calculates the root hash for ommer/uncle headers.
pub fn calculate_ommers_root(ommers: &[Header]) -> B256 {
    // Check if `ommers` list is empty
    if ommers.is_empty() {
        return EMPTY_OMMER_ROOT_HASH;
    }
    // RLP Encode
    let mut ommers_rlp = Vec::new();
    alloy_rlp::encode_list(ommers, &mut ommers_rlp);
    keccak256(ommers_rlp)
}

/// Hashes and sorts account keys, then proceeds to calculating the root hash of the state
/// represented as MPT.
/// See [`state_root_unsorted`] for more info.
pub fn state_root_ref_unhashed<'a, A: Into<Account> + Clone + 'a>(
    state: impl IntoIterator<Item = (&'a Address, &'a A)>,
) -> B256 {
    state_root_unsorted(
        state.into_iter().map(|(address, account)| (keccak256(address), account.clone())),
    )
}

/// Hashes and sorts account keys, then proceeds to calculating the root hash of the state
/// represented as MPT.
/// See [`state_root_unsorted`] for more info.
pub fn state_root_unhashed<A: Into<Account>>(
    state: impl IntoIterator<Item = (Address, A)>,
) -> B256 {
    state_root_unsorted(state.into_iter().map(|(address, account)| (keccak256(address), account)))
}

/// Sorts the hashed account keys and calculates the root hash of the state represented as MPT.
/// See [`state_root`] for more info.
pub fn state_root_unsorted<A: Into<Account>>(state: impl IntoIterator<Item = (B256, A)>) -> B256 {
    state_root(state.into_iter().sorted_unstable_by_key(|(key, _)| *key))
}

/// Calculates the root hash of the state represented as MPT.
///
/// Corresponds to [geth's `deriveHash`](https://github.com/ethereum/go-ethereum/blob/6c149fd4ad063f7c24d726a73bc0546badd1bc73/core/genesis.go#L119).
///
/// # Panics
///
/// If the items are not in sorted order.
pub fn state_root<A: Into<Account>>(state: impl IntoIterator<Item = (B256, A)>) -> B256 {
    let mut hb = HashBuilder::default();
    for (hashed_key, account) in state {
        let account_rlp_buf = alloy_rlp::encode(account.into());
        hb.add_leaf(Nibbles::unpack(hashed_key), &account_rlp_buf);
    }
    hb.root()
}

/// Hashes storage keys, sorts them and them calculates the root hash of the storage trie.
/// See [`storage_root_unsorted`] for more info.
pub fn storage_root_unhashed(storage: impl IntoIterator<Item = (B256, U256)>) -> B256 {
    storage_root_unsorted(storage.into_iter().map(|(slot, value)| (keccak256(slot), value)))
}

/// Sorts and calculates the root hash of account storage trie.
/// See [`storage_root`] for more info.
pub fn storage_root_unsorted(storage: impl IntoIterator<Item = (B256, U256)>) -> B256 {
    storage_root(storage.into_iter().sorted_unstable_by_key(|(key, _)| *key))
}

/// Calculates the root hash of account storage trie.
///
/// # Panics
///
/// If the items are not in sorted order.
pub fn storage_root(storage: impl IntoIterator<Item = (B256, U256)>) -> B256 {
    let mut hb = HashBuilder::default();
    for (hashed_slot, value) in storage {
        hb.add_leaf(Nibbles::unpack(hashed_slot), alloy_rlp::encode_fixed_size(&value).as_ref());
    }
    hb.root()
}
