//! Contains the `AccountChanges` struct, which represents storage, balance, nonce, code changes and
//! read for the account. All changes for a single account, grouped by field type.
//! This eliminates address redundancy across different change types.

use alloc::vec::Vec;
use alloy_primitives::{Address, StorageKey};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::{
    balance_change::BalanceChange, code_change::CodeChange, nonce_change::NonceChange, SlotChanges,
    MAX_SLOTS, MAX_TXS,
};

/// This struct is used to track the changes across accounts in a block.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct AccountChanges {
    /// The address of the account whose changes are stored.
    pub address: Address,
    /// List of slot changes for this account.
    pub storage_changes: Vec<SlotChanges>,
    /// List of storage reads for this account.
    pub storage_reads: Vec<StorageKey>,
    /// List of balance changes for this account.
    pub balance_changes: Vec<BalanceChange>,
    /// List of nonce changes for this account.
    pub nonce_changes: Vec<NonceChange>,
    /// List of code changes for this account.
    pub code_changes: Vec<CodeChange>,
}

impl AccountChanges {
    /// Creates a new `AccountChanges` instance for the given address.
    /// TODO! Needs appropriate method to populate
    pub fn new(address: Address) -> Self {
        Self {
            address,
            storage_changes: Vec::with_capacity(MAX_SLOTS),
            storage_reads: Vec::with_capacity(MAX_SLOTS),
            balance_changes: Vec::with_capacity(MAX_TXS),
            nonce_changes: Vec::with_capacity(MAX_TXS),
            code_changes: Vec::with_capacity(MAX_TXS),
        }
    }

    /// Returns the address of the account.
    #[inline]
    pub const fn address(&self) -> Address {
        self.address
    }

    /// Returns the storage changes for this account.
    #[inline]
    pub fn storage_changes(&self) -> &[SlotChanges] {
        &self.storage_changes
    }

    /// Returns the storage reads for this account.
    #[inline]
    pub fn storage_reads(&self) -> &[StorageKey] {
        &self.storage_reads
    }

    /// Returns the balance changes for this account.
    #[inline]
    pub fn balance_changes(&self) -> &[BalanceChange] {
        &self.balance_changes
    }

    /// Returns the nonce changes for this account.
    #[inline]
    pub fn nonce_changes(&self) -> &[NonceChange] {
        &self.nonce_changes
    }

    /// Returns the code changes for this account.
    #[inline]
    pub fn code_changes(&self) -> &[CodeChange] {
        &self.code_changes
    }

    /// Set the address.
    pub fn with_address(mut self, address: Address) -> Self {
        self.address = address;
        self
    }

    /// Add a storage read slot.
    pub fn with_storage_read(mut self, key: StorageKey) -> Self {
        self.storage_reads.push(key);
        self
    }

    /// Add a storage change (multiple writes to a slot grouped in `SlotChanges`).
    pub fn with_storage_change(mut self, change: SlotChanges) -> Self {
        self.storage_changes.push(change);
        self
    }

    /// Add a balance change.
    pub fn with_balance_change(mut self, change: BalanceChange) -> Self {
        self.balance_changes.push(change);
        self
    }

    /// Add a nonce change.
    pub fn with_nonce_change(mut self, change: NonceChange) -> Self {
        self.nonce_changes.push(change);
        self
    }

    /// Add a code change.
    pub fn with_code_change(mut self, change: CodeChange) -> Self {
        self.code_changes.push(change);
        self
    }

    /// Add multiple storage reads at once.
    pub fn extend_storage_reads<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = StorageKey>,
    {
        self.storage_reads.extend(iter);
        self
    }

    /// Add multiple slot changes at once.
    pub fn extend_storage_changes<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = SlotChanges>,
    {
        self.storage_changes.extend(iter);
        self
    }
}
