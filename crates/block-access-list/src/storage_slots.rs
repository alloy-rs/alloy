//! Contains the `SlotChanges` struct, which represents all changes made to a single storage slot
//! across

use crate::{StorageChange, MAX_TXS};
use alloc::vec::Vec;
use alloy_primitives::StorageKey;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// Represents all changes made to a single storage slot across multiple transactions.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
pub struct SlotChanges {
    /// The storage slot key being modified.
    pub slot: StorageKey,
    /// A list of write operations to this slot, ordered by transaction index.
    pub changes: Vec<StorageChange>,
}

impl SlotChanges {
    /// Creates a new `SlotChanges` instance for the given slot key.
    ///
    /// Preallocates capacity for up to 300,000 changes.
    #[inline]
    pub fn new(slot: StorageKey) -> Self {
        Self { slot, changes: Vec::with_capacity(MAX_TXS) }
    }

    /// Appends a storage change to the list.
    #[inline]
    pub fn push(&mut self, change: StorageChange) {
        self.changes.push(change)
    }

    /// Returns `true` if no changes have been recorded.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Returns the number of changes recorded for this slot.
    #[inline]
    pub fn len(&self) -> usize {
        self.changes.len()
    }
}
