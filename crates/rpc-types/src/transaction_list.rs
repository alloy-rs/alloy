use crate::Transaction;
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

/// A list of transactions, either full, hashes or uncle for uncle blocks pre-merge.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionList<T = Transaction> {
    /// Hashes only.
    Hashes(Vec<B256>),
    /// Full transactions
    Full(Vec<T>),
    /// Special case for uncle response
    Uncle,
}

impl<T> TransactionList<T> {
    /// Check if the enum variant is used for hashes.
    #[inline]
    pub const fn is_hashes(&self) -> bool {
        matches!(self, Self::Hashes(_))
    }

    /// Returns true if the enum variant is used for full transactions.
    #[inline]
    pub const fn is_full(&self) -> bool {
        matches!(self, Self::Full(_))
    }

    /// Returns true if the enum variant is used for an uncle response.
    #[inline]
    pub const fn is_uncle(&self) -> bool {
        matches!(self, Self::Uncle)
    }

    /// Returns an instance of BlockTransactions with the Uncle special case.
    #[inline]
    pub const fn uncle() -> Self {
        Self::Uncle
    }
}
