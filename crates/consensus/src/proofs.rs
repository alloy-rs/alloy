//! Helper function for calculating Merkle proofs and hashes.

use crate::{Header, EMPTY_OMMER_ROOT_HASH};
use alloc::vec::Vec;
use alloy_eips::{eip2718::Encodable2718, eip4895::Withdrawal};
use alloy_primitives::{keccak256, B256};
use alloy_trie::root::{ordered_trie_root, ordered_trie_root_with_encoder};

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
