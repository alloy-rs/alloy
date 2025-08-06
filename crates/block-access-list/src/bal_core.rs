use alloc::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
use alloy_primitives::{map::HashMap, Bytes, StorageKey, StorageValue, TxIndex, U256};
use serde::{Deserialize, Serialize};

/// `StorageAccess` keeps a record of storage_reads and storage_writes as per Eip-7928
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageAccess {
    /// tx_index → read_keys
    pub reads: BTreeMap<TxIndex, BTreeSet<StorageKey>>,
    /// tx_index → key → (pre, post)
    pub writes: BTreeMap<TxIndex, BTreeMap<StorageKey, (StorageValue, StorageValue)>>,
}

/// `BalanceChange` keeps a record of pre_balance and post_balance as per Eip-7928
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceChange {
    /// tx_index → (pre_balance , post_balance)
    pub change: HashMap<TxIndex, (U256, U256)>,
}

/// `NonceChange` keeps a record of post_nonce as per Eip-7928
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonceChange {
    /// tx_index → (pre_nonce , post_nonce)
    pub change: HashMap<TxIndex, (u64, u64)>,
}

/// `CodeChange` keeps a record of post_code as per Eip-7928
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeChange {
    /// tx_index →  post_bytecode
    pub change: HashMap<TxIndex, Bytes>,
}
