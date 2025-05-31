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

/// Response from the `eth/v1/node/version` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionData {
    pub version: String,
}

/// Root response from `/eth/v1/node/identity`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub peer_id: String,
    pub enr: String,
    pub p2p_addresses: Vec<String>,
    pub discovery_addresses: Vec<String>,
    pub metadata: IdentityMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityMetadata {
    pub seq_number: String,
    pub attnets: String,
    pub syncnets: String,
}

/// Response from the `eth/v1/node/peers` endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PeerDirection {
    Inbound,
    Outbound,
}

/// Response from the `eth/v1/node/peers` endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PeerState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Metadata about an individual peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub enr: Option<String>,
    pub last_seen_p2p_address: String,
    pub state: PeerState,
    pub direction: PeerDirection,
}

/// Metadata returned with peer list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersMeta {
    pub count: usize,
}

/// Response from `/eth/v1/node/peers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersResponse {
    pub data: Vec<PeerInfo>,
    pub meta: PeersMeta,
}

/// Response from `/eth/v1/node/peer_count`.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCount {
    #[serde_as(as = "DisplayFromStr")]
    pub disconnected: usize,
    #[serde_as(as = "DisplayFromStr")]
    pub connecting: usize,
    #[serde_as(as = "DisplayFromStr")]
    pub connected: usize,
    #[serde_as(as = "DisplayFromStr")]
    pub disconnecting: usize,
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

    #[test]
    fn test_identity_with_null_enr() {
        let s = r#"
        {
            "peer_id": "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
            "enr": "enr:-IS4QHCYrYZbAKWCBRlAy5zzaDZXJBGkcnh4MHcBFZntXNFrdvJjX04jRzjzCBOonrkTfj499SZuOh8R33Ls8RRcy5wBgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQPKY0yuDUmstAHYpMa2_oxVtw0RW_QAdpzBQA8yWM0xOIN1ZHCCdl8",
            "p2p_addresses": [
              "/ip4/7.7.7.7/tcp/4242/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N"
            ],
            "discovery_addresses": [
              "/ip4/7.7.7.7/udp/30303/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N"
            ],
            "metadata": {
              "seq_number": "1",
              "attnets": "0x0000000000000000",
              "syncnets": "0x0f"
            }
        }
        "#;

        let _result: NodeIdentity = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_peer_count_response() {
        let json = r#"
        {
            "disconnected": "1",
            "connecting": "1",
            "connected": "1",
            "disconnecting": "1"
        }
        "#;

        let _parsed: PeerCount = serde_json::from_str(json).unwrap();
    }

    #[test]
    fn test_peers_response() {
        let json = r#"
        {
            "data": [
                {
                    "peer_id": "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
                    "enr": null,
                    "last_seen_p2p_address": "/ip4/7.7.7.7/tcp/4242/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
                    "state": "disconnected",
                    "direction": "inbound"
                }
            ],
            "meta": {
                "count": 1
            }
        }
        "#;

        let _parsed: PeersResponse = serde_json::from_str(json).unwrap();
    }
}
