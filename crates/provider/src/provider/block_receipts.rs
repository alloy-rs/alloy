use alloy_eips::{BlockId, RpcBlockHash};
use serde::{Serialize, Serializer};

/// The block parameter for an `eth_getBlockReceipts` request.
///
/// Many `eth_getBlockReceipts` implementations expect a bare block-hash string
/// (`"0x.."`) and reject the EIP-1898 object form (`{"blockHash":".."}`) that
/// [`BlockId`] serializes a hash to. This wrapper serializes a
/// [`BlockId::Hash`] as a plain hash string when `require_canonical` is not set,
/// and otherwise (a block number/tag, or a hash that carries `require_canonical`)
/// falls back to the standard [`BlockId`] serialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockReceiptsParams(pub BlockId);

impl From<BlockId> for BlockReceiptsParams {
    fn from(block: BlockId) -> Self {
        Self(block)
    }
}

impl Serialize for BlockReceiptsParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            BlockId::Hash(RpcBlockHash { block_hash, require_canonical: None }) => {
                block_hash.serialize(serializer)
            }
            other => other.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{b256, B256};

    const HASH: B256 = b256!("0xa4c3c1414373ed64c1b80d007d8edab0335193d87788ebfddc59c79a5feb1ec2");

    #[test]
    fn hash_serializes_as_plain_string() {
        let params = BlockReceiptsParams::from(BlockId::from(HASH));
        let json = serde_json::to_value(params).unwrap();
        assert_eq!(
            json,
            serde_json::json!("0xa4c3c1414373ed64c1b80d007d8edab0335193d87788ebfddc59c79a5feb1ec2")
        );
    }

    #[test]
    fn hash_with_require_canonical_keeps_object_form() {
        let block = BlockId::Hash(RpcBlockHash::from_hash(HASH, Some(true)));
        let json = serde_json::to_value(BlockReceiptsParams(block)).unwrap();
        assert!(json.is_object());
        assert_eq!(
            json["blockHash"],
            serde_json::json!("0xa4c3c1414373ed64c1b80d007d8edab0335193d87788ebfddc59c79a5feb1ec2")
        );
        assert_eq!(json["requireCanonical"], serde_json::json!(true));
    }

    #[test]
    fn number_serializes_as_quantity_string() {
        let json =
            serde_json::to_value(BlockReceiptsParams::from(BlockId::number(0x12f7f81))).unwrap();
        assert_eq!(json, serde_json::json!("0x12f7f81"));
    }

    #[test]
    fn tag_serializes_as_string() {
        let json = serde_json::to_value(BlockReceiptsParams::from(BlockId::latest())).unwrap();
        assert_eq!(json, serde_json::json!("latest"));
    }
}
