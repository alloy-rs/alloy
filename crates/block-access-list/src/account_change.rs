//! Contains the `AccountChanges` struct, which represents storage, balance, nonce, code changes and
//! read for the account. All changes for a single account, grouped by field type.
//! This eliminates address redundancy across different change types.

use alloc::vec::Vec;
use alloy_primitives::{Address, StorageKey};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::{
    balance_change::BalanceChanges, code_change::CodeChanges, nonce_change::NonceChanges,
    SlotChanges, MAX_SLOTS, MAX_TXS,
};

/// This struct is used to track the changes across accounts in a block.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize,
)]
pub struct AccountChanges {
    /// The address of the account whoose changes are stored.
    pub address: Address,
    /// List of slot changes for this account.
    pub storage_changes: Vec<SlotChanges>,
    /// List of storage reads for this account.
    pub storage_reads: Vec<StorageKey>,
    /// List of balance changes for this account.
    pub balance_changes: Vec<BalanceChanges>,
    /// List of nonce changes for this account.
    pub nonce_changes: Vec<NonceChanges>,
    /// List of code changes for this account.
    pub code_changes: Vec<CodeChanges>,
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
    pub fn balance_changes(&self) -> &[BalanceChanges] {
        &self.balance_changes
    }

    /// Returns the nonce changes for this account.
    #[inline]
    pub fn nonce_changes(&self) -> &[NonceChanges] {
        &self.nonce_changes
    }

    /// Returns the code changes for this account.
    #[inline]
    pub fn code_changes(&self) -> &[CodeChanges] {
        &self.code_changes
    }
}
