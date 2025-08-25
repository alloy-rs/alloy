//! Contains the `BlockAccessList` struct, which represents a simple list of account changes.

use crate::account_change::AccountChanges;
use alloc::vec::Vec;

/// Vector of account changes.
pub type BlockAccessList = Vec<AccountChanges>;
