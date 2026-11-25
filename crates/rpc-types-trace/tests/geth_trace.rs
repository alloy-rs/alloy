//! Untagged deserialization of top-level [`GethTrace`] responses.

use alloy_rpc_types_trace::geth::{GethTrace, NoopFrame};

const STRUCT_LOGS: &str = include_str!("../test_data/default/structlogs_01.json");
const CALL: &str = include_str!("../test_data/call_tracer/default.json");
const PRESTATE: &str = include_str!("../test_data/pre_state_tracer/default.json");
const PRESTATE_DIFF: &str = include_str!("../test_data/pre_state_tracer/diff_mode.json");
const FLAT_CALL: &str = r#"[{
    "action": {
        "from": "0xc77820eef59629fc8d88154977bc8de8a1b2f4ae",
        "callType": "call",
        "gas": "0x4a0d00",
        "input": "0x12",
        "to": "0x4f4495243837681061c4743b74b3eedf548d56a5",
        "value": "0x0"
    },
    "blockHash": "0xd5ac5043011d4f16dba7841fa760c4659644b78f663b901af4673b679605ed0d",
    "blockNumber": 18557272,
    "result": { "gasUsed": "0x17d337", "output": "0x" },
    "subtraces": 0,
    "traceAddress": [],
    "transactionHash": "0x54160ddcdbfaf98a43a43c328ebd44aa99faa765e0daa93e61145b06815a4071",
    "transactionPosition": 102,
    "type": "call"
}]"#;
const FOUR_BYTE: &str = r#"{"0x27dc297e-128": 1, "0x38cc4831-0": 2}"#;
const STATE_GAS: &str = r#"{"gasUsed": "0x5208", "executionGasUsed": "0x5208", "stateGasUsed": "0x0", "gasRefund": "0x0"}"#;
const EMPTY: &str = "{}";
const JS: &str = r#""custom js tracer result""#;

const fn variant(trace: &GethTrace) -> &'static str {
    match trace {
        GethTrace::Default(_) => "Default",
        GethTrace::Erc7562Tracer(_) => "Erc7562Tracer",
        GethTrace::CallTracer(_) => "CallTracer",
        GethTrace::FlatCallTracer(_) => "FlatCallTracer",
        GethTrace::FourByteTracer(_) => "FourByteTracer",
        GethTrace::PreStateTracer(_) => "PreStateTracer",
        GethTrace::StateGasTracer(_) => "StateGasTracer",
        GethTrace::NoopTracer(_) => "NoopTracer",
        GethTrace::MuxTracer(_) => "MuxTracer",
        GethTrace::JS(_) => "JS",
    }
}

#[test]
fn top_level_variants() {
    let mux =
        format!(r#"{{"4byteTracer": {FOUR_BYTE}, "callTracer": {CALL}, "noopTracer": {{}}}}"#);
    let cases = [
        (STRUCT_LOGS, "Default"),
        (CALL, "CallTracer"),
        (FLAT_CALL, "FlatCallTracer"),
        (FOUR_BYTE, "FourByteTracer"),
        (PRESTATE, "PreStateTracer"),
        (PRESTATE_DIFF, "PreStateTracer"),
        (STATE_GAS, "StateGasTracer"),
        // `{}` is both the noopTracer response and an empty 4byteTracer response.
        (EMPTY, "FourByteTracer"),
        (&mux, "MuxTracer"),
        (JS, "JS"),
    ];

    for (json, expected) in cases {
        let trace: GethTrace = serde_json::from_str(json).unwrap();
        assert_eq!(variant(&trace), expected, "{json}");
    }
}

#[test]
fn noop_tracer_response_converts_to_noop_frame() {
    let trace: GethTrace = serde_json::from_str(EMPTY).unwrap();
    assert_eq!(trace.try_into_noop_frame().unwrap(), NoopFrame::default());

    let trace: GethTrace = serde_json::from_str(FOUR_BYTE).unwrap();
    assert!(trace.try_into_noop_frame().is_err());
}
