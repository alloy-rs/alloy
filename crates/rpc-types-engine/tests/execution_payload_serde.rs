//! Deserialization of execution payloads from JSON.

#![cfg(feature = "serde")]

use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2,
};
use serde::{de::DeserializeOwned, Deserialize};
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

#[test]
fn flattened_execution_payload_preserves_siblings() {
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

    for fields in [
        json!({}),
        json!({"withdrawals": []}),
        json!({"withdrawals": [], "blobGasUsed": "0x0", "excessBlobGas": "0x0"}),
        json!({
            "withdrawals": [], "blobGasUsed": "0x0", "excessBlobGas": "0x0",
            "blockAccessList": "0xc0", "slotNumber": "0x7",
        }),
    ] {
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
