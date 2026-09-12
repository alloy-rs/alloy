use super::*;
use alloy_eips::eip4895::Withdrawal;
use alloy_primitives::{Address, Bloom, Bytes};
use ssz::{Decode, Encode};

fn payload_v1() -> ExecutionPayloadV1 {
    ExecutionPayloadV1 {
        parent_hash: B256::repeat_byte(1),
        fee_recipient: Address::repeat_byte(2),
        state_root: B256::repeat_byte(3),
        receipts_root: B256::repeat_byte(4),
        logs_bloom: Bloom::repeat_byte(5),
        prev_randao: B256::repeat_byte(6),
        block_number: 7,
        gas_limit: 8,
        gas_used: 9,
        timestamp: 10,
        extra_data: Bytes::from_static(&[11, 12]),
        base_fee_per_gas: U256::from(13),
        block_hash: B256::repeat_byte(14),
        transactions: vec![Bytes::from_static(&[15, 16])],
    }
}

fn payload_v2() -> ExecutionPayloadV2 {
    ExecutionPayloadV2 { payload_inner: payload_v1(), withdrawals: vec![Withdrawal::default()] }
}

fn payload_v3() -> ExecutionPayloadV3 {
    ExecutionPayloadV3 { payload_inner: payload_v2(), blob_gas_used: 17, excess_blob_gas: 18 }
}

fn payload_v4() -> ExecutionPayloadV4 {
    ExecutionPayloadV4 {
        payload_inner: payload_v3(),
        block_access_list: Bytes::from_static(&[19, 20]),
        slot_number: 21,
    }
}

fn attributes_cancun() -> PayloadAttributesCancun {
    PayloadAttributesCancun {
        timestamp: 1,
        prev_randao: B256::repeat_byte(2),
        suggested_fee_recipient: Address::repeat_byte(3),
        withdrawals: vec![Withdrawal::default()],
        parent_beacon_block_root: B256::repeat_byte(4),
    }
}

fn state() -> ForkchoiceState {
    ForkchoiceState {
        head_block_hash: B256::repeat_byte(1),
        safe_block_hash: B256::repeat_byte(2),
        finalized_block_hash: B256::repeat_byte(3),
    }
}

fn assert_roundtrip<T>(value: &T)
where
    T: Encode + Decode + PartialEq + core::fmt::Debug,
{
    assert_eq!(T::from_ssz_bytes(&value.as_ssz_bytes()).unwrap(), *value);
}

#[test]
fn execution_payload_envelopes_roundtrip() {
    assert_roundtrip(&ExecutionPayloadEnvelopeParis { payload: payload_v1() });
    assert_roundtrip(&ExecutionPayloadEnvelopeShanghai { payload: payload_v2() });
    assert_roundtrip(&ExecutionPayloadEnvelopeCancun {
        payload: payload_v3(),
        parent_beacon_block_root: B256::repeat_byte(1),
    });
    assert_roundtrip(&ExecutionPayloadEnvelopePrague {
        payload: payload_v3(),
        parent_beacon_block_root: B256::repeat_byte(1),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
    });
    assert_roundtrip(&ExecutionPayloadEnvelopeOsaka {
        payload: payload_v3(),
        parent_beacon_block_root: B256::repeat_byte(1),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
    });
    assert_roundtrip(&ExecutionPayloadEnvelopeAmsterdam {
        payload: payload_v4(),
        parent_beacon_block_root: B256::repeat_byte(1),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
    });
}

#[test]
fn paris_submission_is_a_single_field_container() {
    let payload = payload_v1();
    let payload_bytes = payload.as_ssz_bytes();
    let envelope = ExecutionPayloadEnvelopeParis { payload };
    let encoded = envelope.as_ssz_bytes();
    assert_eq!(&encoded[..4], &4u32.to_le_bytes());
    assert_eq!(&encoded[4..], payload_bytes);
}

#[test]
fn built_payloads_roundtrip() {
    assert_roundtrip(&BuiltPayloadParis { payload: payload_v1(), block_value: U256::from(1) });
    assert_roundtrip(&BuiltPayloadShanghai { payload: payload_v2(), block_value: U256::from(1) });
    assert_roundtrip(&BuiltPayloadCancun {
        execution_payload: payload_v3(),
        block_value: U256::from(1),
        blobs_bundle: BlobsBundleV1::empty(),
        should_override_builder: true,
    });
    assert_roundtrip(&BuiltPayloadPrague {
        payload: payload_v3(),
        block_value: U256::from(1),
        blobs_bundle: BlobsBundleV1::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        should_override_builder: true,
    });
    assert_roundtrip(&BuiltPayloadOsaka {
        payload: payload_v3(),
        block_value: U256::from(1),
        blobs_bundle: BlobsBundleV2::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        should_override_builder: true,
    });
    assert_roundtrip(&BuiltPayloadAmsterdam {
        payload: payload_v4(),
        block_value: U256::from(1),
        blobs_bundle: BlobsBundleV2::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[2, 3])]),
        should_override_builder: true,
    });
}

#[test]
fn shanghai_built_payload_has_no_builder_override() {
    let payload = payload_v2();
    let payload_len = payload.ssz_bytes_len();
    let value = BuiltPayloadShanghai { payload, block_value: U256::from(1) };
    let encoded = value.as_ssz_bytes();

    assert_eq!(&encoded[..4], &36u32.to_le_bytes());
    assert_eq!(encoded.len(), 36 + payload_len);
}

#[test]
fn legacy_built_payload_conversions_preserve_fields() {
    let shanghai = BuiltPayloadShanghai { payload: payload_v2(), block_value: U256::from(1) };
    let legacy = LegacyBuiltPayloadShanghai::from(shanghai.clone());
    assert_eq!(BuiltPayloadShanghai::try_from(legacy).unwrap(), shanghai);

    let prague = BuiltPayloadPrague {
        payload: payload_v3(),
        block_value: U256::from(2),
        blobs_bundle: BlobsBundleV1::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[3, 4])]),
        should_override_builder: true,
    };
    let legacy = LegacyBuiltPayloadPrague::from(prague.clone());
    assert_eq!(BuiltPayloadPrague::from(legacy), prague);

    let osaka = BuiltPayloadOsaka {
        payload: payload_v3(),
        block_value: U256::from(5),
        blobs_bundle: BlobsBundleV2::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[6, 7])]),
        should_override_builder: true,
    };
    let legacy = LegacyBuiltPayloadOsaka::from(osaka.clone());
    assert_eq!(BuiltPayloadOsaka::from(legacy), osaka);

    let amsterdam = BuiltPayloadAmsterdam {
        payload: payload_v4(),
        block_value: U256::from(8),
        blobs_bundle: BlobsBundleV2::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[9, 10])]),
        should_override_builder: true,
    };
    let legacy = LegacyBuiltPayloadAmsterdam::from(amsterdam.clone());
    assert_eq!(BuiltPayloadAmsterdam::from(legacy), amsterdam);
}

#[test]
fn legacy_shanghai_built_payload_rejects_paris_payload() {
    let legacy = LegacyBuiltPayloadShanghai {
        execution_payload: ExecutionPayloadFieldV2::V1(payload_v1()),
        block_value: U256::from(1),
    };
    assert_eq!(
        BuiltPayloadShanghai::try_from(legacy),
        Err(BuiltPayloadConversionError::UnexpectedPayloadFork("Paris"))
    );
}

#[test]
fn prague_requests_precede_should_override_builder() {
    let value = BuiltPayloadPrague {
        payload: payload_v3(),
        block_value: U256::from(1),
        blobs_bundle: BlobsBundleV1::empty(),
        execution_requests: Requests::from_requests([Bytes::from_static(&[0xaa, 0xbb])]),
        should_override_builder: true,
    };
    let encoded = value.as_ssz_bytes();
    assert_eq!(encoded[48], 1);
    assert_eq!(&encoded[encoded.len() - 2..], &[0xaa, 0xbb]);
}

#[test]
fn forkchoice_updates_roundtrip() {
    let paris = PayloadAttributesParis {
        timestamp: 1,
        prev_randao: B256::repeat_byte(2),
        suggested_fee_recipient: Address::repeat_byte(3),
    };
    let shanghai = PayloadAttributesShanghai {
        timestamp: 1,
        prev_randao: B256::repeat_byte(2),
        suggested_fee_recipient: Address::repeat_byte(3),
        withdrawals: vec![Withdrawal::default()],
    };
    let amsterdam = PayloadAttributesAmsterdam {
        timestamp: 1,
        prev_randao: B256::repeat_byte(2),
        suggested_fee_recipient: Address::repeat_byte(3),
        withdrawals: vec![Withdrawal::default()],
        parent_beacon_block_root: B256::repeat_byte(4),
        slot_number: 5,
        target_gas_limit: 6,
    };
    assert_roundtrip(&ForkchoiceUpdateParis {
        forkchoice_state: state(),
        payload_attributes: Optional::some(paris),
    });
    assert_roundtrip(&ForkchoiceUpdateShanghai {
        forkchoice_state: state(),
        payload_attributes: Optional::some(shanghai),
    });
    assert_roundtrip(&ForkchoiceUpdateCancun {
        forkchoice_state: state(),
        payload_attributes: Optional::some(attributes_cancun()),
    });
    assert_roundtrip(&ForkchoiceUpdatePrague {
        forkchoice_state: state(),
        payload_attributes: Optional::some(attributes_cancun()),
    });
    assert_roundtrip(&ForkchoiceUpdateOsaka {
        forkchoice_state: state(),
        payload_attributes: Optional::some(attributes_cancun()),
    });
    assert_roundtrip(&ForkchoiceUpdateAmsterdam {
        forkchoice_state: state(),
        payload_attributes: Optional::some(amsterdam),
        custody_columns: Optional::some(B128::repeat_byte(0xa5)),
    });
}

#[test]
fn payload_attributes_legacy_conversions_preserve_fork_shape() {
    let cancun = attributes_cancun();
    let legacy = LegacyPayloadAttributes::from(cancun.clone());
    assert_eq!(PayloadAttributesCancun::try_from(legacy).unwrap(), cancun);

    let amsterdam = PayloadAttributesAmsterdam {
        timestamp: 1,
        prev_randao: B256::repeat_byte(2),
        suggested_fee_recipient: Address::repeat_byte(3),
        withdrawals: vec![Withdrawal::default()],
        parent_beacon_block_root: B256::repeat_byte(4),
        slot_number: 5,
        target_gas_limit: 6,
    };
    let legacy = LegacyPayloadAttributes::from(amsterdam.clone());
    assert_eq!(PayloadAttributesAmsterdam::try_from(legacy).unwrap(), amsterdam);
}

#[test]
fn payload_attributes_legacy_conversions_reject_loss() {
    let mut legacy = LegacyPayloadAttributes::default();
    assert_eq!(
        PayloadAttributesShanghai::try_from(legacy.clone()),
        Err(PayloadAttributesConversionError::MissingField("withdrawals"))
    );

    legacy.withdrawals = Some(vec![]);
    legacy.parent_beacon_block_root = Some(B256::ZERO);
    assert_eq!(
        PayloadAttributesShanghai::try_from(legacy),
        Err(PayloadAttributesConversionError::UnexpectedField("parent_beacon_block_root"))
    );
}

#[test]
fn every_payload_status_roundtrips() {
    for status in [
        PayloadStatusEnum::Valid,
        PayloadStatusEnum::Invalid { validation_error: "invalid".into() },
        PayloadStatusEnum::Syncing,
        PayloadStatusEnum::Accepted,
    ] {
        let value = PayloadStatus::try_from(LegacyPayloadStatus {
            status,
            latest_valid_hash: Some(B256::ZERO),
        })
        .unwrap();
        assert_eq!(PayloadStatus::from_ssz_bytes(&value.as_ssz_bytes()).unwrap(), value);
    }
}

#[test]
fn payload_status_preserves_absent_invalid_validation_error() {
    let mut bytes = Vec::new();
    let mut encoder = ssz::SszEncoder::container(&mut bytes, 9);
    encoder.append(&1u8);
    encoder.append(&Optional::<B256>::none());
    encoder.append(&Optional::<ValidationError>::none());
    encoder.finalize();

    let decoded = PayloadStatus::from_ssz_bytes(&bytes).unwrap();
    assert!(decoded.validation_error.is_none());
    assert_eq!(decoded.as_ssz_bytes(), bytes);
}

#[test]
fn payload_status_rejects_non_invalid_validation_error() {
    let mut bytes = Vec::new();
    let mut encoder = ssz::SszEncoder::container(&mut bytes, 9);
    encoder.append(&0u8);
    encoder.append(&Optional::<B256>::none());
    encoder.append(&Optional::some(Vec::<u8>::new()));
    encoder.finalize();
    assert!(PayloadStatus::from_ssz_bytes(&bytes).is_err());
}

#[test]
fn payload_status_legacy_conversion_rejects_oversized_error() {
    assert!(PayloadStatus::try_from(LegacyPayloadStatus {
        status: PayloadStatusEnum::Invalid { validation_error: "x".repeat(1025) },
        latest_valid_hash: None,
    })
    .is_err());
}

#[test]
fn forkchoice_response_distinguishes_absent_and_zero_payload_id() {
    let status = PayloadStatus {
        status: PayloadStatusKind::Valid,
        latest_valid_hash: Optional::none(),
        validation_error: Optional::none(),
    };
    let none =
        ForkchoiceUpdateResponse { payload_status: status.clone(), payload_id: Optional::none() };
    let zero = ForkchoiceUpdateResponse {
        payload_status: status,
        payload_id: Optional::some(PayloadId::default()),
    };

    assert_ne!(none.as_ssz_bytes(), zero.as_ssz_bytes());
    assert_roundtrip(&none);
    assert_roundtrip(&zero);
}

#[test]
fn forkchoice_conversion_rejects_accepted() {
    let legacy = LegacyForkchoice::from_status(PayloadStatusEnum::Accepted);
    assert_eq!(
        ForkchoiceUpdateResponse::try_from(legacy),
        Err(ConversionError::AcceptedForkchoice)
    );
}

fn blob_v2(byte: u8) -> BlobAndProofV2 {
    BlobAndProofV2 {
        blob: Box::new(Blob::repeat_byte(byte)),
        proofs: vec![Bytes48::repeat_byte(byte)],
    }
}

#[test]
fn blob_requests_are_single_field_containers() {
    let request = BlobsV1Request { versioned_hashes: vec![B256::repeat_byte(0x42)] };
    let encoded = request.as_ssz_bytes();

    assert_eq!(&encoded[..4], &4u32.to_le_bytes());
    assert_eq!(&encoded[4..], B256::repeat_byte(0x42).as_slice());
    assert_eq!(BlobsV1Request::from_ssz_bytes(&encoded).unwrap(), request);

    let _: BlobsV2Request = BlobsV2Request::from_ssz_bytes(&encoded).unwrap();
    let _: BlobsV3Request = BlobsV3Request::from_ssz_bytes(&encoded).unwrap();
}

#[test]
fn blob_v4_request_roundtrips_bitvector() {
    let request = BlobsV4Request {
        versioned_hashes: vec![B256::repeat_byte(0x11)],
        indices_bitarray: B128::repeat_byte(0xa5),
    };

    assert_roundtrip(&request);
}

#[test]
fn blob_response_conversions_preserve_availability_and_order() {
    let v1 = BlobsV1Response::try_from(vec![None]).unwrap();
    assert!(!v1.entries[0].available);
    assert_eq!(
        v1.entries[0].contents,
        BlobAndProofV1 { blob: Box::new(Blob::ZERO), proof: Bytes48::ZERO }
    );

    let v2 = BlobsV2Response::try_from(vec![blob_v2(1), blob_v2(2)]).unwrap();
    assert!(v2.entries.iter().all(|entry| entry.available));

    let v3 = BlobsV3Response::try_from(vec![Some(blob_v2(1)), None, Some(blob_v2(3))]).unwrap();
    assert_eq!(
        v3.entries.iter().map(|entry| entry.available).collect::<Vec<_>>(),
        [true, false, true]
    );
    assert_eq!(v3.entries[2].contents.blob.as_slice(), Blob::repeat_byte(3).as_slice());

    let legacy_partial = BlobCellsAndProofsV1 {
        blob_cells: vec![Some(Cell::repeat_byte(1)), None],
        proofs: vec![Some(Bytes48::repeat_byte(2)), None],
    };
    let v4 = BlobsV4Response::try_from(vec![None, Some(legacy_partial)]).unwrap();
    assert!(!v4.entries[0].available);
    assert!(v4.entries[1].available);
    assert!(v4.entries[1].contents.blob_cells[0].is_some());
    assert!(v4.entries[1].contents.proofs[1].is_none());
}

#[test]
fn blob_cells_and_proofs_uses_rest_optional() {
    let value = BlobCellsAndProofs {
        blob_cells: vec![Optional::some(Cell::repeat_byte(1))],
        proofs: vec![Optional::some(Bytes48::repeat_byte(2))],
    };
    let encoded = value.as_ssz_bytes();

    assert_eq!(BlobCellsAndProofs::from_ssz_bytes(&encoded).unwrap(), value);
    assert!(!encoded[8..].starts_with(&[1, 0, 0, 0]));
}

#[test]
fn payload_body_requests_are_single_field_containers() {
    let request = BodiesByHashRequest { block_hashes: vec![B256::repeat_byte(0x33)] };
    let encoded = request.as_ssz_bytes();

    assert_eq!(&encoded[..4], &4u32.to_le_bytes());
    assert_eq!(&encoded[4..], B256::repeat_byte(0x33).as_slice());
    assert_eq!(BodiesByHashRequest::from_ssz_bytes(&encoded).unwrap(), request);
}

#[test]
fn payload_body_responses_preserve_availability() {
    let legacy = LegacyExecutionPayloadBodyV1 {
        transactions: vec![Bytes::from_static(&[1, 2, 3])],
        withdrawals: Some(vec![Withdrawal::default()]),
    };
    let response = BodiesResponseShanghai::from_optional_bodies(vec![Some(legacy), None], |body| {
        ExecutionPayloadBodyShanghai::try_from(body).ok()
    })
    .unwrap();

    assert!(response.entries[0].available);
    assert!(!response.entries[1].available);
    assert_roundtrip(&response);
}

#[test]
fn optional_rejects_more_than_one_value() {
    assert!(Optional::<B256>::from_ssz_bytes(&[0; 64]).is_err());
}
#[test]
fn container_payload_bounds() {
    let mut request = ExecutionPayloadEnvelopePrague {
        payload: payload_v3(),
        parent_beacon_block_root: B256::ZERO,
        execution_requests: Requests::new(vec![Bytes::new(); 256]),
    };
    request.payload.payload_inner.withdrawals = vec![Withdrawal::default(); 16];
    request.payload.payload_inner.payload_inner.extra_data = vec![0; 32].into();
    assert_roundtrip(&request);
    request.execution_requests = Requests::new(vec![Bytes::new(); 257]);
    assert!(ExecutionPayloadEnvelopePrague::from_ssz_bytes(&request.as_ssz_bytes()).is_err());
    request.execution_requests = Requests::default();
    request.payload.payload_inner.withdrawals.push(Withdrawal::default());
    assert!(ExecutionPayloadEnvelopePrague::from_ssz_bytes(&request.as_ssz_bytes()).is_err());
    request.payload.payload_inner.withdrawals.clear();
    request.payload.payload_inner.payload_inner.extra_data = vec![0; 33].into();
    assert!(ExecutionPayloadEnvelopePrague::from_ssz_bytes(&request.as_ssz_bytes()).is_err());
    let mut paris = ExecutionPayloadEnvelopeParis { payload: payload_v1() };
    paris.payload.transactions = vec![Bytes::new(); (1 << 20) + 1];
    assert!(ExecutionPayloadEnvelopeParis::from_ssz_bytes(&paris.as_ssz_bytes()).is_err());
}

#[test]
fn forkchoice_container_withdrawal_bounds() {
    for count in [16, 17] {
        let shanghai = ForkchoiceUpdateShanghai {
            forkchoice_state: state(),
            payload_attributes: Optional::some(PayloadAttributesShanghai {
                withdrawals: vec![Withdrawal::default(); count],
                ..Default::default()
            }),
        };
        let cancun = ForkchoiceUpdateCancun {
            forkchoice_state: state(),
            payload_attributes: Optional::some(PayloadAttributesCancun {
                withdrawals: vec![Withdrawal::default(); count],
                ..Default::default()
            }),
        };
        let amsterdam = ForkchoiceUpdateAmsterdam {
            forkchoice_state: state(),
            payload_attributes: Optional::some(PayloadAttributesAmsterdam {
                withdrawals: vec![Withdrawal::default(); count],
                ..Default::default()
            }),
            custody_columns: Optional::none(),
        };
        assert_eq!(
            ForkchoiceUpdateShanghai::from_ssz_bytes(&shanghai.as_ssz_bytes()).is_ok(),
            count == 16
        );
        assert_eq!(
            ForkchoiceUpdateCancun::from_ssz_bytes(&cancun.as_ssz_bytes()).is_ok(),
            count == 16
        );
        assert_eq!(
            ForkchoiceUpdateAmsterdam::from_ssz_bytes(&amsterdam.as_ssz_bytes()).is_ok(),
            count == 16
        );
    }
}

#[test]
fn paris_forkchoice_has_no_offset_inside_fixed_optional() {
    let request = ForkchoiceUpdateParis {
        forkchoice_state: state(),
        payload_attributes: Optional::some(PayloadAttributesParis::default()),
    };
    let bytes = request.as_ssz_bytes();
    assert_eq!(bytes.len(), 160);
    assert_eq!(&bytes[96..100], &100u32.to_le_bytes());
    assert_roundtrip(&request);
}

#[test]
fn blob_and_body_list_bounds() {
    for count in [128, 129] {
        let request = BlobsV1Request { versioned_hashes: vec![B256::ZERO; count] };
        assert_eq!(BlobsV1Request::from_ssz_bytes(&request.as_ssz_bytes()).is_ok(), count == 128);
        let request = BlobsV4Request {
            versioned_hashes: request.versioned_hashes,
            indices_bitarray: B128::ZERO,
        };
        assert_eq!(BlobsV4Request::from_ssz_bytes(&request.as_ssz_bytes()).is_ok(), count == 128);
        let response =
            BlobsResponse { entries: vec![BlobEntry { available: false, contents: 0u8 }; count] };
        assert_eq!(
            BlobsResponse::<u8>::from_ssz_bytes(&response.as_ssz_bytes()).is_ok(),
            count == 128
        );
    }
    for count in [32, 33] {
        let request = BodiesByHashRequest { block_hashes: vec![B256::ZERO; count] };
        assert_eq!(
            BodiesByHashRequest::from_ssz_bytes(&request.as_ssz_bytes()).is_ok(),
            count == 32
        );
        let response: BodiesResponse<ExecutionPayloadBodyParis> =
            BodiesResponse { entries: vec![BodyEntry::unavailable(); count] };
        assert_eq!(
            BodiesResponse::<ExecutionPayloadBodyParis>::from_ssz_bytes(&response.as_ssz_bytes())
                .is_ok(),
            count == 32
        );
    }
    let body = ExecutionPayloadBodyShanghai {
        transactions: vec![],
        withdrawals: vec![Withdrawal::default(); 17],
    };
    assert!(ExecutionPayloadBodyShanghai::from_ssz_bytes(&body.as_ssz_bytes()).is_err());
    let cells = BlobCellsAndProofs { blob_cells: vec![Optional::none(); 129], proofs: vec![] };
    assert!(BlobCellsAndProofs::from_ssz_bytes(&cells.as_ssz_bytes()).is_err());
}

#[test]
fn optional_preserves_fixed_and_variable_list_bytes() {
    assert_eq!(Optional::some(7u64).as_ssz_bytes(), vec![7u64].as_ssz_bytes());
    assert_eq!(Optional::some(Vec::<u8>::new()).as_ssz_bytes(), vec![4, 0, 0, 0]);
    assert_roundtrip(&Optional::some(Vec::<u8>::new()));
    assert!(Optional::<Vec<u8>>::from_ssz_bytes(&[8, 0, 0, 0, 8, 0, 0, 0]).is_err());
    assert!(Optional::<u64>::from_ssz_bytes(&[0; 16]).is_err());
}

#[test]
fn validation_error_bounds_and_empty_presence() {
    assert!(ValidationError::try_from(Bytes::from(vec![b'x'; 1025])).is_err());
    assert!(ValidationError::try_from(Bytes::from_static(&[0xff])).is_err());
    assert!(ValidationError::from_ssz_bytes(&[0xff]).is_err());
    assert!(ValidationError::from_ssz_bytes(&[b'x'; 1025]).is_err());
    for error in [Bytes::new(), Bytes::from(vec![b'x'; 1024])] {
        let status = PayloadStatus {
            status: PayloadStatusKind::Invalid,
            latest_valid_hash: Optional::none(),
            validation_error: Optional::some(ValidationError::try_from(error).unwrap()),
        };
        assert_roundtrip(&status);
        assert!(PayloadStatus::from_ssz_bytes(&status.as_ssz_bytes())
            .unwrap()
            .validation_error
            .is_some());
    }
}

#[test]
fn geth_golden_vectors() {
    // Geth 4797f85ef1add42799b09aa0cef9f715ac48f8d4 beacon/engine/ssz fixtures.
    let fixtures: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("testdata/geth-golden.json")).unwrap();
    fn check<T: Decode + Encode>(data: &str) {
        let bytes = alloy_primitives::hex::decode(data).unwrap();
        assert_eq!(T::from_ssz_bytes(&bytes).unwrap().as_ssz_bytes(), bytes);
    }
    assert_eq!(fixtures.len(), 11);
    for (name, data) in fixtures {
        match name.as_str() {
            "PayloadParis" => check::<ExecutionPayloadParis>(&data),
            "PayloadShanghai" => check::<ExecutionPayloadShanghai>(&data),
            "PayloadCancun" => check::<ExecutionPayloadCancun>(&data),
            "PayloadAmsterdam" => check::<ExecutionPayloadAmsterdam>(&data),
            "AttrsParis" => check::<PayloadAttributesParis>(&data),
            "AttrsShanghai" => check::<PayloadAttributesShanghai>(&data),
            "AttrsCancun" => check::<PayloadAttributesCancun>(&data),
            "AttrsAmsterdam" => check::<PayloadAttributesAmsterdam>(&data),
            "BodyParis" => check::<ExecutionPayloadBodyParis>(&data),
            "BodyCancun" => check::<ExecutionPayloadBodyCancun>(&data),
            "BodyAmsterdam" => check::<ExecutionPayloadBodyAmsterdam>(&data),
            _ => panic!("unknown fixture {name}"),
        }
    }
}

#[test]
fn blob_cells_require_matching_proofs() {
    for value in [
        BlobCellsAndProofs { blob_cells: vec![Optional::none()], proofs: vec![] },
        BlobCellsAndProofs {
            blob_cells: vec![Optional::none()],
            proofs: vec![Optional::some(Bytes48::ZERO)],
        },
        BlobCellsAndProofs {
            blob_cells: vec![Optional::some(Cell::ZERO)],
            proofs: vec![Optional::none()],
        },
    ] {
        assert!(BlobCellsAndProofs::from_ssz_bytes(&value.as_ssz_bytes()).is_err());
    }
}

#[test]
fn body_response_constructor_rejects_oversized_list() {
    assert!(BodiesResponse::<ExecutionPayloadBodyParis>::from_optional_bodies(
        vec![None::<ExecutionPayloadBodyParis>; 33],
        Some
    )
    .is_err());
}
