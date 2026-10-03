//! Deserialization of execution payloads from JSON.

#![cfg(feature = "serde")]

use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

const MAINNET_PAYLOAD: &str = include_str!(
    "../testdata/payload/1752106849375-new_payload-0x98dc28afef51026d21b9933bcabe87611cd1982e876b2b11242ef9ca0c3f79d8.json"
);

fn v1_payload_with(fields: Value) -> Value {
    let mut fixture: Value = serde_json::from_str(MAINNET_PAYLOAD).unwrap();
    let mut payload = fixture["newPayload"]["payload"].take();
    let payload_fields = payload.as_object_mut().unwrap();
    for fork_field in ["withdrawals", "blobGasUsed", "excessBlobGas"] {
        payload_fields.remove(fork_field).unwrap();
    }
    payload_fields.extend(fields.as_object().unwrap().clone());
    payload
}

#[test]
fn execution_payload_variant_selection() {
    let withdrawals = json!([]);
    let blob = "0x20000";
    let bal = "0xc0";
    let slot = "0x10";

    let cases = [
        ("v1", json!({}), Some(1)),
        ("v2", json!({ "withdrawals": withdrawals }), Some(2)),
        (
            "v3",
            json!({ "withdrawals": withdrawals, "blobGasUsed": blob, "excessBlobGas": blob }),
            Some(3),
        ),
        (
            "v4",
            json!({
                "withdrawals": withdrawals, "blobGasUsed": blob, "excessBlobGas": blob,
                "blockAccessList": bal, "slotNumber": slot,
            }),
            Some(4),
        ),
        (
            "v3 + blockAccessList",
            json!({
                "withdrawals": withdrawals, "blobGasUsed": blob, "excessBlobGas": blob,
                "blockAccessList": bal,
            }),
            None,
        ),
        (
            "v3 + slotNumber",
            json!({
                "withdrawals": withdrawals, "blobGasUsed": blob, "excessBlobGas": blob,
                "slotNumber": slot,
            }),
            None,
        ),
        ("v1 + v4 fields", json!({ "blockAccessList": bal, "slotNumber": slot }), None),
        ("v1 + blockAccessList", json!({ "blockAccessList": bal }), None),
        ("v1 + slotNumber", json!({ "slotNumber": slot }), None),
        (
            "v2 + v4 fields",
            json!({ "withdrawals": withdrawals, "blockAccessList": bal, "slotNumber": slot }),
            None,
        ),
        (
            "v2 + blockAccessList",
            json!({ "withdrawals": withdrawals, "blockAccessList": bal }),
            None,
        ),
        ("v2 + slotNumber", json!({ "withdrawals": withdrawals, "slotNumber": slot }), None),
        ("v1 + v3 fields", json!({ "blobGasUsed": blob, "excessBlobGas": blob }), None),
        ("v2 + blobGasUsed", json!({ "withdrawals": withdrawals, "blobGasUsed": blob }), None),
        ("v2 + excessBlobGas", json!({ "withdrawals": withdrawals, "excessBlobGas": blob }), None),
        (
            "v2 + incomplete v3 + v4 fields",
            json!({
                "withdrawals": withdrawals, "blobGasUsed": blob,
                "blockAccessList": bal, "slotNumber": slot,
            }),
            None,
        ),
    ];

    let mut mismatches = Vec::new();
    for (name, fields, expected) in cases {
        let result = serde_json::from_value::<ExecutionPayload>(v1_payload_with(fields));
        let version = result.as_ref().ok().map(|payload| match payload {
            ExecutionPayload::V1(_) => 1,
            ExecutionPayload::V2(_) => 2,
            ExecutionPayload::V3(_) => 3,
            ExecutionPayload::V4(_) => 4,
        });
        if version != expected {
            mismatches.push(format!("{name}: expected {expected:?}, got {version:?}"));
        }

        if let Ok(ExecutionPayload::V4(payload)) = result {
            assert_eq!(payload.block_access_list.as_ref(), [0xc0]);
            assert_eq!(payload.slot_number, 0x10);
            assert_eq!(payload.payload_inner.blob_gas_used, 0x20000);
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

fn accepts<T: DeserializeOwned>(json: &str) -> bool {
    serde_json::from_str::<T>(json).is_ok()
}

#[test]
fn execution_payload_field_handling() {
    const V2_FIELDS: [&str; 15] = [
        "parentHash",
        "feeRecipient",
        "stateRoot",
        "receiptsRoot",
        "logsBloom",
        "prevRandao",
        "blockNumber",
        "gasLimit",
        "gasUsed",
        "timestamp",
        "extraData",
        "baseFeePerGas",
        "blockHash",
        "transactions",
        "withdrawals",
    ];

    let v2 = v1_payload_with(json!({ "withdrawals": [] }));
    let duplicate_field =
        v2.to_string().replacen('{', &format!("{{\"blockNumber\":{},", v2["blockNumber"]), 1);
    let sequence = Value::Array(V2_FIELDS.iter().map(|field| v2[field].clone()).collect());

    // ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2
    let cases = [
        ("v2", v2.to_string(), [true, true, true, true]),
        (
            "unknown field",
            v1_payload_with(json!({ "withdrawals": [], "unknownField": "0x1" })).to_string(),
            [false, true, false, true],
        ),
        (
            "null withdrawals",
            v1_payload_with(json!({ "withdrawals": null })).to_string(),
            [false, true, true, false],
        ),
        (
            "null blobGasUsed",
            v1_payload_with(json!({ "withdrawals": [], "blobGasUsed": null })).to_string(),
            [false, true, false, true],
        ),
        ("duplicate field", duplicate_field, [false; 4]),
        ("sequence", sequence.to_string(), [false, false, true, true]),
    ];

    let mut mismatches = Vec::new();
    for (name, json, expected) in cases {
        let accepted = [
            accepts::<ExecutionPayload>(&json),
            accepts::<ExecutionPayloadFieldV2>(&json),
            accepts::<ExecutionPayloadInputV2>(&json),
            accepts::<ExecutionPayloadV2>(&json),
        ];
        if accepted != expected {
            mismatches.push(format!("{name}: expected {expected:?}, got {accepted:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}
