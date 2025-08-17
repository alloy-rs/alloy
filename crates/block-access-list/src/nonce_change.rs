//! Contains the `NonceChange` struct, which represents a new nonce for an account.
//! Single code change: `tx_index` -> `new_nonce`

use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::BlockAccessIndex;

/// This struct is used to track the new nonce of accounts in a block.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
pub struct NonceChanges {
    /// The index of bal that stores this nonce change.
    pub block_access_index: BlockAccessIndex,
    /// The new code of the account.
    pub new_nonce: u64,
}

impl NonceChanges {
    /// Creates a new `NonceChange`.
    pub const fn new(block_access_index: BlockAccessIndex, new_nonce: u64) -> Self {
        Self { block_access_index, new_nonce }
    }

    /// Returns the bal index.
    pub const fn block_access_index(&self) -> BlockAccessIndex {
        self.block_access_index
    }

    /// Returns the new nonce.
    pub const fn new_nonce(&self) -> u64 {
        self.new_nonce
    }
}
