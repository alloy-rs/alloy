//! Block sidecars RPC types.

use alloy_eips::eip4844::BlobTransactionSidecar;
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

/// Block sidecar representation
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockSidecar {
    /// Transaction sidecar.
    #[serde(default)]
    pub blob_sidecar: BlobTransactionSidecar,
    /// Block hash.
    #[serde(default)]
    pub block_hash: B256,
    /// Block number.
    #[serde(default, with = "alloy_serde::quantity")]
    pub block_number: u64,
    /// Transaction hash.
    #[serde(default)]
    pub tx_hash: B256,
    /// Transaction index.
    #[serde(default, with = "alloy_serde::quantity")]
    pub tx_index: u64,
}

#[test]
fn test_block_sidecar() {
    let block_sidecar = BlockSidecar {
        blob_sidecar: BlobTransactionSidecar::default(),
        block_hash: B256::random(),
        block_number: 1024,
        tx_hash: B256::random(),
        tx_index: 1024,
    };

    let serialized = serde_json::to_string(&block_sidecar).unwrap();
    println!("{}", serialized);
    let deserialized: BlockSidecar = serde_json::from_str(&serialized).unwrap();
    assert_eq!(block_sidecar, deserialized);
}
