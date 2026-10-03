//! Deserialization of engine API execution payloads from JSON.

use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadFieldV2, ExecutionPayloadInputV2, ExecutionPayloadV2,
    ExecutionPayloadV3, ExecutionPayloadV4,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::hint::black_box;

const FIXTURES: [&str; 5] = [
    include_str!("../testdata/payload/1752106789277-new_payload-0xcc4efa5c8aa75a17d26ca6b00645252284594312ed4b5c49f8e855eb2310847c.json"),
    include_str!("../testdata/payload/1752106800691-new_payload-0x906c3b7c84f7fbedd1e98b7464c24aa3243c94726dd9aa715c4765c5d26cbf2e.json"),
    include_str!("../testdata/payload/1752106812611-new_payload-0x43e0383a0472e84398346dd517467ec21e0d786b3ac96ccbc3e6283f87af0b5f.json"),
    include_str!("../testdata/payload/1752106836642-new_payload-0x347077ee96ee1cea7149c2adfb0f933ee6096607602dde80383eeb2627059c31.json"),
    include_str!("../testdata/payload/1752106849375-new_payload-0x98dc28afef51026d21b9933bcabe87611cd1982e876b2b11242ef9ca0c3f79d8.json"),
];

/// Number of times the transactions of all fixtures are repeated in the large payload.
const LARGE_PAYLOAD_REPEAT: usize = 8;

fn v3_payloads() -> Vec<Value> {
    FIXTURES
        .iter()
        .map(|fixture| {
            serde_json::from_str::<Value>(fixture).unwrap()["newPayload"]["payload"].take()
        })
        .collect()
}

fn bench_payload<T: DeserializeOwned>(c: &mut Criterion, ty: &str, input: &str, json: &str) {
    let mut group = c.benchmark_group("engine_payload_deser");
    group.throughput(Throughput::Bytes(json.len() as u64));
    group.bench_with_input(BenchmarkId::new(ty, input), json, |b, json| {
        b.iter(|| serde_json::from_str::<T>(black_box(json)).unwrap())
    });
    group.finish();
}

/// All inputs are derived from the mainnet fixture with the largest JSON encoding: `v2` drops the
/// blob gas fields, `v4` adds a 64 KiB block access list and a slot number, and `v3_large` repeats
/// the transactions of all fixtures [`LARGE_PAYLOAD_REPEAT`] times.
fn payload_deser(c: &mut Criterion) {
    let payloads = v3_payloads();

    let v3 = payloads.iter().max_by_key(|payload| payload.to_string().len()).unwrap().clone();

    let mut v2 = v3.clone();
    let v2_fields = v2.as_object_mut().unwrap();
    v2_fields.remove("blobGasUsed");
    v2_fields.remove("excessBlobGas");

    let mut v4 = v3.clone();
    v4.as_object_mut().unwrap().extend([
        ("blockAccessList".to_string(), json!(format!("0x{}", "ab".repeat(64 * 1024)))),
        ("slotNumber".to_string(), json!("0xc2a8f0")),
    ]);

    let mut large = v3.clone();
    let transactions: Vec<Value> = payloads
        .iter()
        .flat_map(|payload| payload["transactions"].as_array().unwrap().clone())
        .collect();
    large["transactions"] =
        (0..LARGE_PAYLOAD_REPEAT).flat_map(|_| transactions.iter().cloned()).collect();

    let v2 = v2.to_string();
    let v3 = v3.to_string();
    let v4 = v4.to_string();
    let large = large.to_string();

    bench_payload::<ExecutionPayloadV2>(c, "ExecutionPayloadV2", "v2", &v2);
    bench_payload::<ExecutionPayloadInputV2>(c, "ExecutionPayloadInputV2", "v2", &v2);
    bench_payload::<ExecutionPayloadFieldV2>(c, "ExecutionPayloadFieldV2", "v2", &v2);
    bench_payload::<ExecutionPayload>(c, "ExecutionPayload", "v2", &v2);

    bench_payload::<ExecutionPayloadV3>(c, "ExecutionPayloadV3", "v3", &v3);
    bench_payload::<ExecutionPayload>(c, "ExecutionPayload", "v3", &v3);

    bench_payload::<ExecutionPayloadV3>(c, "ExecutionPayloadV3", "v3_large", &large);
    bench_payload::<ExecutionPayload>(c, "ExecutionPayload", "v3_large", &large);

    bench_payload::<ExecutionPayloadV4>(c, "ExecutionPayloadV4", "v4", &v4);
    bench_payload::<ExecutionPayload>(c, "ExecutionPayload", "v4", &v4);
}

criterion_group!(benches, payload_deser);
criterion_main!(benches);
