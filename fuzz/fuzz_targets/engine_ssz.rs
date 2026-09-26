#![no_main]

use alloy_primitives::B256;
use alloy_rpc_types_engine::ssz_engine_types::*;
use libfuzzer_sys::fuzz_target;
use ssz::{Decode, Encode};

fn assert_roundtrip<T>(data: &[u8], name: &str)
where
    T: Decode + Encode,
{
    let Ok(decoded) = T::from_ssz_bytes(data) else {
        return;
    };

    assert_eq!(decoded.as_ssz_bytes(), data, "{name}: encode(decode(bytes)) != bytes");
}

fn raw_roundtrip_properties(data: &[u8]) {
    // Status and Engine API v2 Optional encodings.
    assert_roundtrip::<PayloadStatusKind>(data, "PayloadStatusKind");
    assert_roundtrip::<ValidationError>(data, "ValidationError");
    assert_roundtrip::<Optional<B256>>(data, "Optional<B256>");
    assert_roundtrip::<Optional<ValidationError>>(data, "Optional<ValidationError>");
    assert_roundtrip::<PayloadStatus>(data, "PayloadStatus");
    assert_roundtrip::<ForkchoiceUpdateResponse>(data, "ForkchoiceUpdateResponse");

    // Fork-specific payload attributes and get-payload responses.
    assert_roundtrip::<PayloadAttributesParis>(data, "PayloadAttributesParis");
    assert_roundtrip::<PayloadAttributesShanghai>(data, "PayloadAttributesShanghai");
    assert_roundtrip::<PayloadAttributesCancun>(data, "PayloadAttributesCancun");
    assert_roundtrip::<PayloadAttributesAmsterdam>(data, "PayloadAttributesAmsterdam");
    assert_roundtrip::<BuiltPayloadParis>(data, "BuiltPayloadParis");
    assert_roundtrip::<BuiltPayloadShanghai>(data, "BuiltPayloadShanghai");
    assert_roundtrip::<BuiltPayloadCancun>(data, "BuiltPayloadCancun");
    assert_roundtrip::<BuiltPayloadPrague>(data, "BuiltPayloadPrague");
    assert_roundtrip::<BuiltPayloadOsaka>(data, "BuiltPayloadOsaka");
    assert_roundtrip::<BuiltPayloadAmsterdam>(data, "BuiltPayloadAmsterdam");

    // Fork-specific new-payload and forkchoice-update requests.
    assert_roundtrip::<ExecutionPayloadEnvelopeParis>(data, "ExecutionPayloadEnvelopeParis");
    assert_roundtrip::<ExecutionPayloadEnvelopeShanghai>(data, "ExecutionPayloadEnvelopeShanghai");
    assert_roundtrip::<ExecutionPayloadEnvelopeCancun>(data, "ExecutionPayloadEnvelopeCancun");
    assert_roundtrip::<ExecutionPayloadEnvelopePrague>(data, "ExecutionPayloadEnvelopePrague");
    assert_roundtrip::<ExecutionPayloadEnvelopeAmsterdam>(
        data,
        "ExecutionPayloadEnvelopeAmsterdam",
    );
    assert_roundtrip::<ForkchoiceUpdateParis>(data, "ForkchoiceUpdateParis");
    assert_roundtrip::<ForkchoiceUpdateShanghai>(data, "ForkchoiceUpdateShanghai");
    assert_roundtrip::<ForkchoiceUpdateCancun>(data, "ForkchoiceUpdateCancun");
    assert_roundtrip::<ForkchoiceUpdateAmsterdam>(data, "ForkchoiceUpdateAmsterdam");

    // Historical body requests and responses.
    assert_roundtrip::<ExecutionPayloadBodyParis>(data, "ExecutionPayloadBodyParis");
    assert_roundtrip::<ExecutionPayloadBodyShanghai>(data, "ExecutionPayloadBodyShanghai");
    assert_roundtrip::<ExecutionPayloadBodyAmsterdam>(data, "ExecutionPayloadBodyAmsterdam");
    assert_roundtrip::<BodiesByHashRequest>(data, "BodiesByHashRequest");
    assert_roundtrip::<BodiesResponseParis>(data, "BodiesResponseParis");
    assert_roundtrip::<BodiesResponseShanghai>(data, "BodiesResponseShanghai");
    assert_roundtrip::<BodiesResponseAmsterdam>(data, "BodiesResponseAmsterdam");

    // Blob request and response versions with distinct wire schemas.
    assert_roundtrip::<BlobsV1Request>(data, "BlobsV1Request");
    assert_roundtrip::<BlobsV4Request>(data, "BlobsV4Request");
    assert_roundtrip::<BlobCellsAndProofs>(data, "BlobCellsAndProofs");
    assert_roundtrip::<BlobsV1Response>(data, "BlobsV1Response");
    assert_roundtrip::<BlobsV2Response>(data, "BlobsV2Response");
    assert_roundtrip::<BlobsV4Response>(data, "BlobsV4Response");
}

fuzz_target!(|data: &[u8]| {
    raw_roundtrip_properties(data);
});
