//! Block sidecars RPC types.

use alloy_eips::eip4844::BlobTransactionSidecar;
use alloy_primitives::B256;

/// Block sidecar representation
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BlockSidecar {
    /// Transaction sidecar.
    #[cfg_attr(feature = "serde", serde(default))]
    pub blob_sidecar: BlobTransactionSidecar,
    /// Block hash.
    #[cfg_attr(feature = "serde", serde(default))]
    pub block_hash: B256,
    /// Block number.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub block_number: u64,
    /// Transaction hash.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tx_hash: B256,
    /// Transaction index.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub tx_index: u64,
}

#[test]
#[cfg(feature = "serde")]
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
