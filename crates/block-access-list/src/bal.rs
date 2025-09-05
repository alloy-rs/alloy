//! Contains the `BlockAccessList` struct, which represents a simple list of account changes.

use crate::account_change::AccountChanges;
use alloc::vec::Vec;

/// Vector of account changes.
pub type BlockAccessList = Vec<AccountChanges>;

#[cfg(test)]
mod tests {
    use alloy_primitives::{keccak256, StorageKey, U256};

    #[test]
    fn test_storage() {
        let key = U256::from(1);
        let bkey = StorageKey::from(U256::from(1));
        let key_hash = keccak256(alloy_rlp::encode(key));
        let bkey_hash = keccak256(alloy_rlp::encode(bkey));
        // println!("Key hash for {:?}: {:?}", key, key_hash);
        // println!("Bkey hash for {:?}: {:?}", bkey, bkey_hash);
        assert_ne!(key_hash, bkey_hash);
    }
}
