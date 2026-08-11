//! Contains types related to the Inclusion lists that will be used by in the engine API RPC
//! definitions.

use alloy_primitives::{Address, B256};
use alloy_rpc_types_engine::PayloadStatusEnum;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::fmt;

/// This structure contains the result of processing an `engine_newInclusionListV1` RPC call.
///
/// From the spec:
///
/// ### InclusionListStatusV1
///
/// This structure contains the result of processing an inclusion list. The fields are encoded as
/// follows:
/// - `status`: `enum` - `"VALID" | "INVALID" | "SYNCING" | "ACCEPTED"`
/// - `validationError`: `String|null` - a message providing additional details on the validation
///   error if the payload is classified as `INVALID`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListStatusV1 {
    /// The status of the payload.
    #[serde(flatten)]
    pub status: PayloadStatusEnum,
}

impl InclusionListStatusV1 {
    /// Initializes a new inclusion list status.
    pub const fn new(status: PayloadStatusEnum) -> Self {
        Self { status }
    }

    /// Returns true if the payload status is syncing.
    pub const fn is_syncing(&self) -> bool {
        self.status.is_syncing()
    }

    /// Returns true if the payload status is valid.
    pub const fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    /// Returns true if the payload status is invalid.
    pub const fn is_invalid(&self) -> bool {
        self.status.is_invalid()
    }
}

impl fmt::Display for InclusionListStatusV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InclusionListStatusV1 {{ status: {} }}", self.status)
    }
}

impl Serialize for InclusionListStatusV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("status", self.status.as_str())?;
        map.serialize_entry("validationError", &self.status.validation_error())?;
        map.end()
    }
}

/// This is an individual entry in the inclusion list summary, representing a transaction that
/// should be included in this block or the next block.
///
/// From the spec:
///
/// ### InclusionListSummaryEntryV1
///
/// This structure contains the details of each inclusion list entry.
///
/// - `address` : `DATA`, 20 Bytes
/// - `nonce` : `QUANTITY`, 64 Bits
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListSummaryEntryV1 {
    /// The address of the inclusion list entry.
    pub address: Address,
    /// The nonce of the inclusion list entry.
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
}

impl fmt::Display for InclusionListSummaryEntryV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InclusionListSummaryEntryV1 {{ address: {}, nonce: {} }}",
            self.address, self.nonce
        )
    }
}

/// This structure contains the inclusion list summary input to the `engine_newInclusionListV1` RPC
/// call.
///
/// ### InclusionListSummaryV1
///
/// This structure contains the inclusion list summary.
///
/// - `slot` : `QUANTITY`, 64 Bits
/// - `proposer_index`: `QUANTITY`, 64 Bits
/// - `parent_hash`: `DATA`, 32 Bytes
/// - `summary`: `Array of InclusionListSummaryEntryV1`, Array of entries that must be satisfied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListSummaryV1 {
    /// The slot of the inclusion list summary.
    #[serde(with = "alloy_serde::quantity")]
    pub slot: u64,
    /// The proposer index of the inclusion list summary.
    #[serde(with = "alloy_serde::quantity")]
    pub proposer_index: u64,
    /// The parent hash of the inclusion list summary.
    pub parent_hash: B256,
    /// The summary of the inclusion list summary.
    pub summary: Vec<InclusionListSummaryEntryV1>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex::FromHex;
    use serde_json::json;

    #[test]
    fn inclusion_list_status_v1_serialization() {
        let status = InclusionListStatusV1::new(PayloadStatusEnum::Valid);
        let json = json!({
            "status": "VALID",
            "validationError": null,
        });
        assert_eq!(serde_json::to_value(status).unwrap(), json);
    }

    #[test]
    fn inclusion_list_entry_v1_serialization() {
        let entry = InclusionListSummaryEntryV1 {
            address: Address::from_hex("0x0000000000000000000000000000000000000042").unwrap(),
            nonce: 42,
        };
        let json = json!({
            "address": "0x0000000000000000000000000000000000000042",
            "nonce": "0x2a",
        });
        assert_eq!(serde_json::to_value(entry).unwrap(), json);
    }

    #[test]
    fn inclusion_list_summary_v1_serialization() {
        let summary = InclusionListSummaryV1 {
            slot: 42,
            proposer_index: 42,
            parent_hash: B256::from_hex(
                "0x2222222222222222222222222222222222222222222222222222222222222222",
            )
            .unwrap(),
            summary: vec![
                InclusionListSummaryEntryV1 {
                    address: Address::from_hex("0x0000000000000000000000000000000000000042")
                        .unwrap(),
                    nonce: 42,
                },
                InclusionListSummaryEntryV1 {
                    address: Address::from_hex("0x0000000000000000000000000000000000000043")
                        .unwrap(),
                    nonce: 43,
                },
            ],
        };
        let json = json!({
            "slot": "0x2a",
            "proposerIndex": "0x2a",
            "parentHash": "0x2222222222222222222222222222222222222222222222222222222222222222",
            "summary": [
                {
                    "address": "0x0000000000000000000000000000000000000042",
                    "nonce": "0x2a",
                },
                {
                    "address": "0x0000000000000000000000000000000000000043",
                    "nonce": "0x2b",
                },
            ],
        });
        assert_eq!(serde_json::to_value(summary).unwrap(), json);
    }
}
