//! Deserialization of execution payloads from JSON.

#![cfg(feature = "serde")]

use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use test_case::test_case;

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

const BLOB: &str = "0x20000";
const BAL: &str = "0xc0";
const SLOT: &str = "0x10";

#[test_case(json!({}) => Some(1); "v1")]
#[test_case(json!({ "withdrawals": [] }) => Some(2); "v2")]
#[test_case(
    json!({ "withdrawals": [], "blobGasUsed": BLOB, "excessBlobGas": BLOB }) => Some(3);
    "v3"
)]
#[test_case(
    json!({
        "withdrawals": [], "blobGasUsed": BLOB, "excessBlobGas": BLOB,
        "blockAccessList": BAL, "slotNumber": SLOT,
    }) => Some(4);
    "v4"
)]
#[test_case(
    json!({
        "withdrawals": [], "blobGasUsed": BLOB, "excessBlobGas": BLOB,
        "blockAccessList": BAL,
    }) => None;
    "v3 + blockAccessList"
)]
#[test_case(
    json!({
        "withdrawals": [], "blobGasUsed": BLOB, "excessBlobGas": BLOB,
        "slotNumber": SLOT,
    }) => None;
    "v3 + slotNumber"
)]
#[test_case(json!({ "blockAccessList": BAL, "slotNumber": SLOT }) => None; "v1 + v4 fields")]
#[test_case(json!({ "blockAccessList": BAL }) => None; "v1 + blockAccessList")]
#[test_case(json!({ "slotNumber": SLOT }) => None; "v1 + slotNumber")]
#[test_case(
    json!({ "withdrawals": [], "blockAccessList": BAL, "slotNumber": SLOT }) => None;
    "v2 + v4 fields"
)]
#[test_case(
    json!({ "withdrawals": [], "blockAccessList": BAL }) => None;
    "v2 + blockAccessList"
)]
#[test_case(json!({ "withdrawals": [], "slotNumber": SLOT }) => None; "v2 + slotNumber")]
#[test_case(json!({ "blobGasUsed": BLOB, "excessBlobGas": BLOB }) => None; "v1 + v3 fields")]
#[test_case(json!({ "withdrawals": [], "blobGasUsed": BLOB }) => None; "v2 + blobGasUsed")]
#[test_case(json!({ "withdrawals": [], "excessBlobGas": BLOB }) => None; "v2 + excessBlobGas")]
#[test_case(
    json!({
        "withdrawals": [], "blobGasUsed": BLOB,
        "blockAccessList": BAL, "slotNumber": SLOT,
    }) => None;
    "v2 + incomplete v3 + v4 fields"
)]
fn execution_payload_variant_selection(fields: Value) -> Option<u8> {
    let result = serde_json::from_value::<ExecutionPayload>(v1_payload_with(fields));
    let version = result.as_ref().ok().map(|payload| match payload {
        ExecutionPayload::V1(_) => 1,
        ExecutionPayload::V2(_) => 2,
        ExecutionPayload::V3(_) => 3,
        ExecutionPayload::V4(_) => 4,
    });

    if let Ok(ExecutionPayload::V4(payload)) = result {
        assert_eq!(payload.block_access_list.as_ref(), [0xc0]);
        assert_eq!(payload.slot_number, 0x10);
        assert_eq!(payload.payload_inner.blob_gas_used, 0x20000);
    }
    version
}

fn accepts<T: DeserializeOwned>(json: &str) -> bool {
    serde_json::from_str::<T>(json).is_ok()
}

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

fn v2_payload() -> Value {
    v1_payload_with(json!({ "withdrawals": [] }))
}

fn v2_with_duplicate_field() -> String {
    let v2 = v2_payload();
    v2.to_string().replacen('{', &format!("{{\"blockNumber\":{},", v2["blockNumber"]), 1)
}

fn v2_as_sequence() -> String {
    let v2 = v2_payload();
    Value::Array(V2_FIELDS.iter().map(|field| v2[field].clone()).collect()).to_string()
}

// ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2
#[test_case(v2_payload().to_string() => [true, true, true, true]; "v2")]
#[test_case(
    v1_payload_with(json!({ "withdrawals": [], "unknownField": "0x1" })).to_string()
    => [false, true, false, true];
    "unknown field"
)]
#[test_case(
    v1_payload_with(json!({ "withdrawals": null })).to_string() => [false, true, true, false];
    "null withdrawals"
)]
#[test_case(
    v1_payload_with(json!({ "withdrawals": [], "blobGasUsed": null })).to_string()
    => [false, true, false, true];
    "null blobGasUsed"
)]
#[test_case(v2_with_duplicate_field() => [false; 4]; "duplicate field")]
#[test_case(v2_as_sequence() => [false, false, true, true]; "sequence")]
fn execution_payload_field_handling(json: String) -> [bool; 4] {
    [
        accepts::<ExecutionPayload>(&json),
        accepts::<ExecutionPayloadFieldV2>(&json),
        accepts::<ExecutionPayloadInputV2>(&json),
        accepts::<ExecutionPayloadV2>(&json),
    ]
}

#[test_case(json!({}); "v1")]
#[test_case(json!({"withdrawals": []}); "v2")]
#[test_case(json!({"withdrawals": [], "blobGasUsed": "0x0", "excessBlobGas": "0x0"}); "v3")]
#[test_case(
    json!({
        "withdrawals": [], "blobGasUsed": "0x0", "excessBlobGas": "0x0",
        "blockAccessList": "0xc0", "slotNumber": "0x7",
    });
    "v4"
)]
fn flattened_execution_payload_preserves_siblings(fields: Value) {
    #[derive(Debug, Deserialize)]
    struct Extension {
        metadata: Value,
    }

    #[derive(Debug, Deserialize)]
    struct PayloadFirst {
        #[serde(flatten)]
        payload: ExecutionPayload,
        #[serde(flatten)]
        extension: Extension,
    }

    #[derive(Debug, Deserialize)]
    struct PayloadLast {
        #[serde(flatten)]
        extension: Extension,
        #[serde(flatten)]
        payload: ExecutionPayload,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StrictOuter {
        #[serde(flatten)]
        payload: ExecutionPayload,
        metadata: Value,
    }

    let original = v1_payload_with(fields);
    let expected = serde_json::from_value::<ExecutionPayload>(original.clone()).unwrap();
    for metadata in [json!(null), json!(true), json!({"nested": [1, 2]})] {
        let mut input = original.clone();
        input["metadata"] = metadata.clone();
        let first = serde_json::from_value::<PayloadFirst>(input.clone()).unwrap();
        let last = serde_json::from_str::<PayloadLast>(&input.to_string()).unwrap();
        assert_eq!(first.payload, expected);
        assert_eq!(last.payload, expected);
        assert_eq!(first.extension.metadata, metadata);
        assert_eq!(last.extension.metadata, metadata);
        let outer = serde_json::from_value::<StrictOuter>(input.clone()).unwrap();
        assert_eq!(outer.payload, expected);
        assert_eq!(outer.metadata, metadata);
        input["unknown"] = json!(true);
        assert!(serde_json::from_value::<StrictOuter>(input).is_err());
    }
}

#[test]
fn execution_payload_fork_field_combinations() {
    let fork_fields = [
        ("withdrawals", json!([])),
        ("blobGasUsed", json!("0x0")),
        ("excessBlobGas", json!("0x0")),
        ("blockAccessList", json!("0xc0")),
        ("slotNumber", json!("0x7")),
    ];
    // Each fork field is absent, valid, null, or invalid. Only complete fork prefixes are valid.
    for combination in 0..4_u32.pow(fork_fields.len() as u32) {
        let mut input = v1_payload_with(json!({}));
        let mut states = [0; 5];
        let mut remaining = combination;
        for (index, (field, valid)) in fork_fields.iter().enumerate() {
            let state = remaining % 4;
            remaining /= 4;
            states[index] = state;
            match state {
                0 => {}
                1 => input[*field] = valid.clone(),
                2 => input[*field] = Value::Null,
                _ => input[*field] = json!(false),
            }
        }
        let expected_version = match states {
            [0, 0, 0, 0, 0] => Some(1),
            [1, 0, 0, 0, 0] => Some(2),
            [1, 1, 1, 0, 0] => Some(3),
            [1, 1, 1, 1, 1] => Some(4),
            _ => None,
        };
        for result in [
            serde_json::from_value::<ExecutionPayload>(input.clone()),
            serde_json::from_str::<ExecutionPayload>(&input.to_string()),
        ] {
            let version = result.as_ref().ok().map(|payload| match payload {
                ExecutionPayload::V1(_) => 1,
                ExecutionPayload::V2(_) => 2,
                ExecutionPayload::V3(_) => 3,
                ExecutionPayload::V4(_) => 4,
            });
            assert_eq!(version, expected_version, "fork states: {states:?}");
            if let Ok(payload) = result {
                assert_eq!(serde_json::to_value(payload).unwrap(), input);
            }
        }
    }
}
