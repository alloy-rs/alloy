//! Contains the `BlockAccessList` struct, which represents a simple list of account changes.

use crate::account_change::AccountChanges;
use alloc::vec::Vec;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// This struct is used to store `account_changes` in a block.
#[derive(
    Debug, Clone, Default, PartialEq, RlpDecodable, RlpEncodable, Eq, Serialize, Deserialize,
)]
pub struct BlockAccessList {
    /// List of account changes in the block.
    pub account_changes: Vec<AccountChanges>,
}

impl BlockAccessList {
    /// Creates a new `BlockAccessList` instance.
    /// TODO! Needs appropriate method to populate
    pub fn new(account_changes: Vec<AccountChanges>) -> Self {
        Self { account_changes }
    }

    /// Returns the account changes in the block.
    #[inline]
    pub fn account_changes(&self) -> &[AccountChanges] {
        &self.account_changes
    }
}
