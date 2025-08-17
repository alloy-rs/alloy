//! Contains the `CodeChange` struct, which represents a new code for an account.
//! Single code change: `tx_index` -> `new_code`
use alloy_primitives::Bytes;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::BlockAccessIndex;

/// This struct is used to track the new codes of accounts in a block.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
pub struct CodeChanges {
    /// The index of bal that stores this code change.
    pub block_access_index: BlockAccessIndex,
    /// The new code of the account.
    pub new_code: Bytes,
}
impl CodeChanges {
    /// Creates a new `CodeChange`.
    pub fn new(block_access_index: BlockAccessIndex) -> Self {
        Self { block_access_index, new_code: Default::default() }
    }

    /// Returns the bal index.
    #[inline]
    pub const fn block_access_index(&self) -> BlockAccessIndex {
        self.block_access_index
    }

    /// Returns the new code.
    #[inline]
    pub const fn new_code(&self) -> &Bytes {
        &self.new_code
    }
}
