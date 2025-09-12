//! Contains the `StorageChange` struct, which represents a single storage write operation within a
//! transaction.

use alloy_primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::BlockAccessIndex;

/// Represents a single storage write operation within a transaction.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct StorageChange {
    /// Index of the bal that stores the performed write.
    #[serde(rename = "txIndex", with = "alloy_serde::quantity")]
    pub block_access_index: BlockAccessIndex,
    /// The new value written to the storage slot.
    #[serde(rename = "postValue")]
    pub new_value: B256,
}

impl StorageChange {
    /// Creates a new `StorageChange`.
    #[inline]
    pub const fn new(block_access_index: BlockAccessIndex, new_value: B256) -> Self {
        Self { block_access_index, new_value }
    }

    /// Returns true if the new value is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.new_value == B256::ZERO
    }

    /// Returns true if this change was made by the given transaction.
    #[inline]
    pub const fn is_from_tx(&self, block_index: BlockAccessIndex) -> bool {
        self.block_access_index == block_index
    }

    /// Returns a copy with a different storage value.
    #[inline]
    pub const fn with_value(&self, value: B256) -> Self {
        Self { block_access_index: self.block_access_index, new_value: value }
    }
}
