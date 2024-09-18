//! Types for the `debug` API.

use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the execution witness of a block. Contains an optional map of state preimages.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWitness {
    /// Map of all hashed trie nodes to their preimages that were required during the execution of
    /// the block, including during state root recomputation.
    pub witness: HashMap<B256, Bytes>,
    /// Map of all hashed account addresses and storage slots to their preimages (unhashed account
    /// addresses and storage slots, respectively) that were required during the execution of the
    /// block. during the execution of the block.
    pub state_preimages: Option<HashMap<B256, Bytes>>,
}
