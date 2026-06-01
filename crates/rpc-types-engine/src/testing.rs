//! Testing namespace types for building a block in a single call.
//!
//! This follows the `testing_buildBlockV1` specification.

use crate::PayloadAttributes;
use alloc::vec::Vec;
use alloy_primitives::{Bytes, B256};

/// Capability string for `testing_buildBlockV1`.
pub const TESTING_BUILD_BLOCK_V1: &str = "testing_buildBlockV1";

/// Request payload for `testing_buildBlockV1`.
///
/// See the [Execution API `testing_buildBlockV1` specification][spec].
///
/// [spec]: https://github.com/ethereum/execution-apis/blob/main/src/testing/testing_buildBlockV1.yaml
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TestingBuildBlockRequestV1 {
    /// Parent block hash of the block to build.
    pub parent_block_hash: B256,
    /// Payload attributes.
    pub payload_attributes: PayloadAttributes,
    /// Raw signed transactions to force-include in order.
    pub transactions: Vec<Bytes>,
    /// Optional extra data for the block header.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub extra_data: Option<Bytes>,
}

#[cfg(feature = "serde")]
/// Deserializes from either the camelCase object form produced by [`serde::Serialize`] or the
/// positional JSON-RPC params form defined by the execution-apis spec.
///
/// `transactions: null` and omitted transactions are both represented as an empty transaction
/// vector because this type stores transactions as `Vec<Bytes>`.
impl<'de> serde::Deserialize<'de> for TestingBuildBlockRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RequestObject {
            parent_block_hash: B256,
            payload_attributes: PayloadAttributes,
            #[serde(default)]
            transactions: Option<Vec<Bytes>>,
            #[serde(default)]
            extra_data: Option<Bytes>,
        }

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Request {
            Object(RequestObject),
            Params4(B256, PayloadAttributes, Option<Vec<Bytes>>, Option<Bytes>),
            Params3(B256, PayloadAttributes, Option<Vec<Bytes>>),
            Params2(B256, PayloadAttributes),
        }

        Ok(match Request::deserialize(deserializer)? {
            Request::Object(request) => Self {
                parent_block_hash: request.parent_block_hash,
                payload_attributes: request.payload_attributes,
                transactions: request.transactions.unwrap_or_default(),
                extra_data: request.extra_data,
            },
            Request::Params4(parent_block_hash, payload_attributes, transactions, extra_data) => {
                Self {
                    parent_block_hash,
                    payload_attributes,
                    transactions: transactions.unwrap_or_default(),
                    extra_data,
                }
            }
            Request::Params3(parent_block_hash, payload_attributes, transactions) => Self {
                parent_block_hash,
                payload_attributes,
                transactions: transactions.unwrap_or_default(),
                extra_data: None,
            },
            Request::Params2(parent_block_hash, payload_attributes) => Self {
                parent_block_hash,
                payload_attributes,
                transactions: Vec::new(),
                extra_data: None,
            },
        })
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use serde_json::json;
    use similar_asserts::assert_eq;

    fn parent_block_hash() -> B256 {
        "0xaf51811799f22260e5b4e1f95504dae760505f102dcb2e9ca7d897d8a40124a1".parse().unwrap()
    }

    fn payload_attributes_json() -> serde_json::Value {
        json!({
            "parentBeaconBlockRoot": B256::ZERO,
            "prevRandao": B256::ZERO,
            "suggestedFeeRecipient": Address::ZERO,
            "timestamp": "0x1ce",
            "withdrawals": [],
        })
    }

    fn payload_attributes() -> PayloadAttributes {
        serde_json::from_value(payload_attributes_json()).unwrap()
    }

    #[test]
    fn deserialize_testing_build_block_request_object() {
        let expected = TestingBuildBlockRequestV1 {
            parent_block_hash: parent_block_hash(),
            payload_attributes: payload_attributes(),
            transactions: vec![Bytes::from(vec![0x12, 0x34])],
            extra_data: Some(Bytes::default()),
        };
        let value = serde_json::to_value(&expected).unwrap();

        let actual: TestingBuildBlockRequestV1 = serde_json::from_value(value).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_testing_build_block_request_params_with_null_transactions() {
        let value = json!([
            "0xaf51811799f22260e5b4e1f95504dae760505f102dcb2e9ca7d897d8a40124a1",
            payload_attributes_json(),
            null,
            "0x"
        ]);

        let actual: TestingBuildBlockRequestV1 = serde_json::from_value(value).unwrap();

        assert_eq!(
            actual,
            TestingBuildBlockRequestV1 {
                parent_block_hash: parent_block_hash(),
                payload_attributes: payload_attributes(),
                transactions: Vec::new(),
                extra_data: Some(Bytes::default()),
            }
        );
    }

    #[test]
    fn deserialize_testing_build_block_request_params_with_transactions() {
        let value = json!([
            "0xaf51811799f22260e5b4e1f95504dae760505f102dcb2e9ca7d897d8a40124a1",
            payload_attributes_json(),
            ["0x1234"]
        ]);

        let actual: TestingBuildBlockRequestV1 = serde_json::from_value(value).unwrap();

        assert_eq!(
            actual,
            TestingBuildBlockRequestV1 {
                parent_block_hash: parent_block_hash(),
                payload_attributes: payload_attributes(),
                transactions: vec![Bytes::from(vec![0x12, 0x34])],
                extra_data: None,
            }
        );
    }
}
