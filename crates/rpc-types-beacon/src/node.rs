//! Types for <https://ethereum.github.io/beacon-APIs/#/Node>
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the `eth/v1/node/syncing` endpoint.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    #[serde_as(as = "DisplayFromStr")]
    pub head_slot: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub sync_distance: usize,
    pub is_syncing: bool,
    #[serde(default)]
    pub is_optimistic: bool,
    #[serde(default)]
    pub el_offline: bool,
}

/// Response from the `eth/v1/node/health` endpoint.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Ready,
    Syncing,
    NotInitialized,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status() {
        let s = r#"{
    "head_slot": "1",
    "sync_distance": "1",
    "is_syncing": true,
    "is_optimistic": true,
    "el_offline": true
  }"#;

        let _sync_status: SyncStatus = serde_json::from_str(s).unwrap();
    }
}
