use alloy_consensus::{
    BlobTransactionSidecar, BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, Block,
    BlockBody, EthereumReceipt, EthereumTxEnvelope, Extended, Header, Receipt, ReceiptEnvelope,
    ReceiptWithBloom, Receipts, SignableTransaction, Signed, TxEip1559, TxEip2930, TxEip4844,
    TxEip4844WithSidecar, TxEip7702, TxEnvelope, TxLegacy,
};
use alloy_consensus::transaction::PooledTransaction;
use alloy_consensus_any::AnyReceiptEnvelope;
use alloy_eips::{
    eip4844::{Blob, BlobAndProofV1, BlobAndProofV2, Bytes48},
    eip2718::{Decodable2718, Encodable2718},
    eip4895::Withdrawals,
    eip7594::{Cell, CELLS_PER_EXT_BLOB, Decodable7594, Encodable7594},
    eip7685::Requests,
};
use alloy_primitives::{B128, Bloom, Signature, U256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader, EMPTY_STRING_CODE};
use alloy_rpc_types_engine::{
    ssz_engine_types::*, BlobsBundleV1, BlobsBundleV2, ExecutionPayloadEnvelopeV3,
    ForkchoiceState, PayloadId,
};
use ssz::{Decode as SszDecode, Encode as SszEncode};
use std::{fmt::Debug, fs, path::Path};

include!("types.rs");

type BasicTxEnvelope = EthereumTxEnvelope<TxEip4844>;
type ExtendedTransaction = Extended<TxEip1559, TxEip2930>;
type VariantPooledTransaction =
    EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarVariant>>;

fn write_seed(corpus_dir: &Path, name: &str, bytes: &[u8]) {
    let path = corpus_dir.join(name);
    fs::write(&path, bytes).unwrap_or_else(|err| panic!("failed to write {name}: {err}"));
}

fn seed<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + Encodable + Decodable,
{
    let bytes = alloy_rlp::encode(&value);
    let mut input = bytes.as_slice();
    let decoded = T::decode(&mut input).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert!(input.is_empty(), "{name}: decode left trailing bytes");
    assert_eq!(decoded, value, "{name}: decode(encode(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

fn seed_2718<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + Encodable2718 + Decodable2718,
{
    let mut bytes = Vec::with_capacity(value.encode_2718_len());
    value.encode_2718(&mut bytes);
    let decoded = T::decode_2718_exact(&bytes).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert_eq!(decoded, value, "{name}: decode_2718(encode_2718(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

fn seed_7594<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + Encodable7594 + Decodable7594,
{
    let bytes = value.encoded_7594();
    let mut input = bytes.as_slice();
    let decoded = T::decode_7594(&mut input).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert!(input.is_empty(), "{name}: decode_7594 left trailing bytes");
    assert_eq!(decoded, value, "{name}: decode_7594(encode_7594(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

fn seed_ssz<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + SszDecode + SszEncode,
{
    let bytes = value.as_ssz_bytes();
    let decoded = T::from_ssz_bytes(&bytes).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert_eq!(decoded, value, "{name}: decode(encode(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

fn execution_payload_paris() -> ExecutionPayloadParis {
    ExecutionPayloadParis {
        parent_hash: B256::ZERO,
        fee_recipient: Address::ZERO,
        state_root: B256::ZERO,
        receipts_root: B256::ZERO,
        logs_bloom: Bloom::ZERO,
        prev_randao: B256::ZERO,
        block_number: 0,
        gas_limit: 0,
        gas_used: 0,
        timestamp: 0,
        extra_data: Bytes::from_static(b"ssz"),
        base_fee_per_gas: U256::ZERO,
        block_hash: B256::ZERO,
        transactions: vec![Bytes::from_static(b"\x02\xc0")],
    }
}

fn execution_payload_shanghai() -> ExecutionPayloadShanghai {
    ExecutionPayloadShanghai {
        payload_inner: execution_payload_paris(),
        withdrawals: vec![Default::default()],
    }
}

fn execution_payload_cancun() -> ExecutionPayloadCancun {
    ExecutionPayloadCancun {
        payload_inner: execution_payload_shanghai(),
        blob_gas_used: 0,
        excess_blob_gas: 0,
    }
}

fn execution_payload_amsterdam() -> ExecutionPayloadAmsterdam {
    ExecutionPayloadAmsterdam {
        payload_inner: execution_payload_cancun(),
        block_access_list: Bytes::from_static(b"\xc0"),
        slot_number: 0,
    }
}

fn engine_ssz_seeds() {
    let corpus_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("corpus")
        .join("engine_ssz");
    fs::create_dir_all(&corpus_dir).expect("failed to create Engine SSZ corpus directory");

    seed_ssz(&corpus_dir, "status-valid-kind", PayloadStatusKind::Valid);
    seed_ssz(
        &corpus_dir,
        "validation-error",
        ValidationError::try_from(Bytes::from_static(b"invalid payload")).unwrap(),
    );
    seed_ssz(&corpus_dir, "optional-hash-none", Optional::<B256>::none());
    seed_ssz(
        &corpus_dir,
        "optional-hash-some",
        Optional::some(B256::repeat_byte(0x42)),
    );
    seed_ssz(
        &corpus_dir,
        "payload-status-valid",
        PayloadStatus {
            status: PayloadStatusKind::Valid,
            latest_valid_hash: Optional::some(B256::ZERO),
            validation_error: Optional::none(),
        },
    );
    seed_ssz(
        &corpus_dir,
        "payload-status-invalid",
        PayloadStatus {
            status: PayloadStatusKind::Invalid,
            latest_valid_hash: Optional::none(),
            validation_error: Optional::some(
                ValidationError::try_from(Bytes::from_static(b"invalid payload")).unwrap(),
            ),
        },
    );
    seed_ssz(
        &corpus_dir,
        "forkchoice-response",
        ForkchoiceUpdateResponse {
            payload_status: PayloadStatus {
                status: PayloadStatusKind::Syncing,
                latest_valid_hash: Optional::none(),
                validation_error: Optional::none(),
            },
            payload_id: Optional::some(PayloadId::default()),
        },
    );

    seed_ssz(&corpus_dir, "attributes-paris", PayloadAttributesParis::default());
    seed_ssz(&corpus_dir, "attributes-shanghai", PayloadAttributesShanghai::default());
    seed_ssz(&corpus_dir, "attributes-cancun", PayloadAttributesCancun::default());
    seed_ssz(&corpus_dir, "attributes-amsterdam", PayloadAttributesAmsterdam::default());

    let requests = Requests::new(vec![Bytes::from_static(b"\x00\x01")]);
    seed_ssz(
        &corpus_dir,
        "built-payload-paris",
        BuiltPayloadParis { payload: execution_payload_paris(), block_value: U256::ZERO },
    );
    seed_ssz(
        &corpus_dir,
        "built-payload-shanghai",
        BuiltPayloadShanghai { payload: execution_payload_shanghai(), block_value: U256::ZERO },
    );
    seed_ssz(
        &corpus_dir,
        "built-payload-cancun",
        ExecutionPayloadEnvelopeV3 {
            execution_payload: execution_payload_cancun(),
            block_value: U256::ZERO,
            blobs_bundle: BlobsBundleV1::default(),
            should_override_builder: false,
        },
    );
    seed_ssz(
        &corpus_dir,
        "built-payload-prague",
        BuiltPayloadPrague {
            payload: execution_payload_cancun(),
            block_value: U256::ZERO,
            blobs_bundle: BlobsBundleV1::default(),
            execution_requests: requests.clone(),
            should_override_builder: false,
        },
    );
    seed_ssz(
        &corpus_dir,
        "built-payload-osaka",
        BuiltPayloadOsaka {
            payload: execution_payload_cancun(),
            block_value: U256::ZERO,
            blobs_bundle: BlobsBundleV2::default(),
            execution_requests: requests.clone(),
            should_override_builder: false,
        },
    );
    seed_ssz(
        &corpus_dir,
        "built-payload-amsterdam",
        BuiltPayloadAmsterdam {
            payload: execution_payload_amsterdam(),
            block_value: U256::ZERO,
            blobs_bundle: BlobsBundleV2::default(),
            execution_requests: requests.clone(),
            should_override_builder: false,
        },
    );

    seed_ssz(
        &corpus_dir,
        "new-payload-paris",
        ExecutionPayloadEnvelopeParis::from(execution_payload_paris()),
    );
    seed_ssz(
        &corpus_dir,
        "new-payload-shanghai",
        ExecutionPayloadEnvelopeShanghai::from(execution_payload_shanghai()),
    );
    seed_ssz(
        &corpus_dir,
        "new-payload-cancun",
        ExecutionPayloadEnvelopeCancun::from((execution_payload_cancun(), B256::ZERO)),
    );
    seed_ssz(
        &corpus_dir,
        "new-payload-prague",
        ExecutionPayloadEnvelopePrague::from((
            execution_payload_cancun(),
            B256::ZERO,
            requests.clone(),
        )),
    );
    seed_ssz(
        &corpus_dir,
        "new-payload-amsterdam",
        ExecutionPayloadEnvelopeAmsterdam::from((
            execution_payload_amsterdam(),
            B256::ZERO,
            requests,
        )),
    );

    seed_ssz(
        &corpus_dir,
        "forkchoice-paris",
        ForkchoiceUpdateParis {
            forkchoice_state: ForkchoiceState::default(),
            payload_attributes: Optional::some(PayloadAttributesParis::default()),
        },
    );
    seed_ssz(
        &corpus_dir,
        "forkchoice-shanghai",
        ForkchoiceUpdateShanghai {
            forkchoice_state: ForkchoiceState::default(),
            payload_attributes: Optional::some(PayloadAttributesShanghai::default()),
        },
    );
    seed_ssz(
        &corpus_dir,
        "forkchoice-cancun",
        ForkchoiceUpdateCancun {
            forkchoice_state: ForkchoiceState::default(),
            payload_attributes: Optional::some(PayloadAttributesCancun::default()),
        },
    );
    seed_ssz(
        &corpus_dir,
        "forkchoice-amsterdam",
        ForkchoiceUpdateAmsterdam {
            forkchoice_state: ForkchoiceState::default(),
            payload_attributes: Optional::some(PayloadAttributesAmsterdam::default()),
            custody_columns: Optional::some(B128::ZERO),
        },
    );

    let body_paris = ExecutionPayloadBodyParis {
        transactions: vec![Bytes::from_static(b"\xc0")],
    };
    let body_shanghai = ExecutionPayloadBodyShanghai {
        transactions: vec![Bytes::from_static(b"\x02\xc0")],
        withdrawals: vec![Default::default()],
    };
    let body_amsterdam = ExecutionPayloadBodyAmsterdam {
        transactions: vec![Bytes::from_static(b"\x04\xc0")],
        withdrawals: vec![Default::default()],
        block_access_list: Bytes::from_static(b"\xc0"),
    };
    seed_ssz(&corpus_dir, "body-paris", body_paris.clone());
    seed_ssz(&corpus_dir, "body-shanghai", body_shanghai.clone());
    seed_ssz(&corpus_dir, "body-amsterdam", body_amsterdam.clone());
    seed_ssz(
        &corpus_dir,
        "bodies-by-hash",
        BodiesByHashRequest { block_hashes: vec![B256::ZERO] },
    );
    seed_ssz(
        &corpus_dir,
        "bodies-response-paris",
        BodiesResponse { entries: vec![BodyEntry::available(body_paris), BodyEntry::unavailable()] },
    );
    seed_ssz(
        &corpus_dir,
        "bodies-response-shanghai",
        BodiesResponse {
            entries: vec![BodyEntry::available(body_shanghai), BodyEntry::unavailable()],
        },
    );
    seed_ssz(
        &corpus_dir,
        "bodies-response-amsterdam",
        BodiesResponse {
            entries: vec![BodyEntry::available(body_amsterdam), BodyEntry::unavailable()],
        },
    );

    seed_ssz(
        &corpus_dir,
        "blobs-v1-request",
        BlobsV1Request { versioned_hashes: vec![B256::ZERO] },
    );
    seed_ssz(
        &corpus_dir,
        "blobs-v4-request",
        BlobsV4Request { versioned_hashes: vec![B256::ZERO], indices_bitarray: B128::ZERO },
    );
    let cells_and_proofs = BlobCellsAndProofs {
        blob_cells: vec![Optional::some(Cell::ZERO)],
        proofs: vec![Optional::some(Bytes48::ZERO)],
    };
    seed_ssz(&corpus_dir, "blob-cells-and-proofs", cells_and_proofs.clone());
    seed_ssz(
        &corpus_dir,
        "blobs-v1-response",
        BlobsResponse {
            entries: vec![BlobEntry {
                available: true,
                contents: BlobAndProofV1 { blob: Box::new(Blob::ZERO), proof: Bytes48::ZERO },
            }],
        },
    );
    seed_ssz(
        &corpus_dir,
        "blobs-v2-response",
        BlobsResponse {
            entries: vec![BlobEntry {
                available: true,
                contents: BlobAndProofV2 { blob: Box::new(Blob::ZERO), proofs: vec![] },
            }],
        },
    );
    seed_ssz(
        &corpus_dir,
        "blobs-v4-response",
        BlobsResponse {
            entries: vec![BlobEntry { available: true, contents: cells_and_proofs }],
        },
    );
}

/// `trailing(no_gaps)` treats a payload item as the next optional value rather than a `None`
/// sentinel. A zero-length string is consequently rejected for this fixed-size field.
fn assert_no_explicit_none(value: &OptionalTypes) {
    let encoded = alloy_rlp::encode(value);
    let mut payload = encoded.as_slice();
    let header =
        RlpHeader::decode(&mut payload).expect("optional types must encode as an RLP list");
    assert!(header.list);
    assert_eq!(payload.len(), header.payload_length);

    let mut non_canonical = Vec::with_capacity(encoded.len() + 1);
    RlpHeader { list: true, payload_length: header.payload_length + 1 }.encode(&mut non_canonical);
    non_canonical.extend_from_slice(payload);
    non_canonical.push(EMPTY_STRING_CODE);

    assert!(OptionalTypes::decode(&mut non_canonical.as_slice()).is_err());
}

fn basic(extreme: bool) -> BasicTypes {
    BasicTypes {
        boolean: extreme,
        u8_value: if extreme { u8::MAX } else { 0 },
        u16_value: if extreme { u16::MAX } else { 0 },
        u32_value: if extreme { u32::MAX } else { 0 },
        u64_value: if extreme { u64::MAX } else { 0 },
        u128_value: if extreme { u128::MAX } else { 0 },
        bytes: if extreme { Bytes::from(vec![0xff; 56]) } else { Bytes::new() },
        string: if extreme { "RLP \u{1f600} boundary".into() } else { String::new() },
        address: if extreme { Address::repeat_byte(0xff) } else { Address::ZERO },
        hash: if extreme { B256::repeat_byte(0xff) } else { B256::ZERO },
    }
}

fn main() {
    let corpus_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("corpus")
        .join("consensus_rlp");
    fs::create_dir_all(&corpus_dir).expect("failed to create corpus directory");

    seed(&corpus_dir, "basic-zero", basic(false));
    seed(&corpus_dir, "basic-extreme", basic(true));
    seed(
        &corpus_dir,
        "vectors-empty",
        VectorTypes { integers: vec![], byte_strings: vec![], strings: vec![], basics: vec![] },
    );
    seed(
        &corpus_dir,
        "vectors-populated",
        VectorTypes {
            integers: vec![0, 1, 127, 128, u64::MAX],
            byte_strings: vec![
                Bytes::new(),
                Bytes::from_static(b"hello"),
                Bytes::from(vec![1; 56]),
            ],
            strings: vec![String::new(), "ascii".into(), "\u{1f600}".into()],
            basics: vec![basic(false), basic(true)],
        },
    );
    seed(
        &corpus_dir,
        "arrays",
        ArrayTypes {
            bytes: [0xff; 32],
            integers: vec![0, 128, u32::MAX],
            hashes: vec![B256::ZERO, B256::repeat_byte(0xff)],
        },
    );
    seed(
        &corpus_dir,
        "nested",
        NestedTypes {
            basic: basic(true),
            vectors: VectorTypes {
                integers: vec![1, 2, 3],
                byte_strings: vec![Bytes::from(vec![0x42; 56])],
                strings: vec!["nested".into()],
                basics: vec![basic(false)],
            },
            arrays: ArrayTypes {
                bytes: [0x42; 32],
                integers: vec![1, 2, 3],
                hashes: vec![B256::repeat_byte(1), B256::repeat_byte(2)],
            },
        },
    );
    let optional_none = OptionalTypes {
        required: 0,
        bytes: Bytes::new(),
        optional_hash: None,
        optional_values: None,
    };
    assert_no_explicit_none(&optional_none);
    seed(&corpus_dir, "optional-none", optional_none);
    seed(
        &corpus_dir,
        "optional-hash",
        OptionalTypes {
            required: 1,
            bytes: Bytes::from_static(b"optional"),
            optional_hash: Some(B256::repeat_byte(0x42)),
            optional_values: None,
        },
    );
    seed(
        &corpus_dir,
        "optional-all",
        OptionalTypes {
            required: u64::MAX,
            bytes: Bytes::from(vec![0xff; 56]),
            optional_hash: Some(B256::repeat_byte(0xff)),
            optional_values: Some(vec![0, 1, 127, 128, u64::MAX]),
        },
    );

    seed(&corpus_dir, "header-default", Header::default());
    seed(&corpus_dir, "block-body-empty", BlockBody::<BasicTxEnvelope>::default());
    seed(
        &corpus_dir,
        "block-empty",
        Block { header: Header::default(), body: BlockBody::<BasicTxEnvelope>::default() },
    );
    seed(&corpus_dir, "tx-legacy-default", TxLegacy::default());
    seed(&corpus_dir, "tx-eip2930-default", TxEip2930::default());
    seed(&corpus_dir, "tx-eip1559-default", TxEip1559::default());
    seed(&corpus_dir, "tx-eip4844-default", TxEip4844::default());
    seed(&corpus_dir, "tx-eip7702-default", TxEip7702::default());
    seed(&corpus_dir, "extended-built-in", ExtendedTransaction::BuiltIn(TxEip1559::default()));
    seed(&corpus_dir, "extended-other", ExtendedTransaction::Other(TxEip2930::default()));
    seed(&corpus_dir, "sidecar-eip4844-empty", BlobTransactionSidecar::default());
    seed(&corpus_dir, "sidecar-eip7594-empty", BlobTransactionSidecarEip7594::default());
    seed(&corpus_dir, "sidecar-variant-eip4844-empty", BlobTransactionSidecarVariant::default());
    seed(
        &corpus_dir,
        "sidecar-variant-eip7594-empty",
        BlobTransactionSidecarVariant::Eip7594(BlobTransactionSidecarEip7594::default()),
    );
    let sidecar_eip4844 = BlobTransactionSidecar {
        blobs: vec![Blob::ZERO],
        commitments: vec![Bytes48::ZERO],
        proofs: vec![Bytes48::ZERO],
    };
    seed(&corpus_dir, "sidecar-eip4844-one-blob", sidecar_eip4844.clone());
    seed_7594(
        &corpus_dir,
        "sidecar-eip4844-one-blob-fields-7594",
        sidecar_eip4844.clone(),
    );
    seed(
        &corpus_dir,
        "sidecar-variant-eip4844-one-blob",
        BlobTransactionSidecarVariant::Eip4844(sidecar_eip4844),
    );
    let sidecar_eip7594 = BlobTransactionSidecarEip7594::new(
        vec![Blob::ZERO],
        vec![Bytes48::ZERO],
        vec![Bytes48::ZERO; CELLS_PER_EXT_BLOB],
    );
    seed(&corpus_dir, "sidecar-eip7594-one-blob", sidecar_eip7594.clone());
    seed_7594(
        &corpus_dir,
        "sidecar-eip7594-one-blob-fields-7594",
        sidecar_eip7594.clone(),
    );
    let signature = Signature::test_signature();
    let pooled_eip7594 = PooledTransaction::Eip4844(Signed::new_unhashed(
        TxEip4844WithSidecar::from_tx_and_sidecar(
            TxEip4844::default(),
            sidecar_eip7594.clone(),
        ),
        signature,
    ));
    seed(&corpus_dir, "pooled-eip7594-one-blob-network", pooled_eip7594.clone());
    seed_2718(&corpus_dir, "pooled-eip7594-one-blob-direct", pooled_eip7594);
    let variant_pooled = VariantPooledTransaction::Eip4844(Signed::new_unhashed(
        TxEip4844WithSidecar::from_tx_and_sidecar(
            TxEip4844::default(),
            BlobTransactionSidecarVariant::Eip7594(sidecar_eip7594.clone()),
        ),
        signature,
    ));
    seed(&corpus_dir, "variant-pooled-eip7594-network", variant_pooled.clone());
    seed_2718(&corpus_dir, "variant-pooled-eip7594-direct", variant_pooled);
    seed(
        &corpus_dir,
        "sidecar-variant-eip7594-one-blob",
        BlobTransactionSidecarVariant::Eip7594(sidecar_eip7594.clone()),
    );
    seed_7594(
        &corpus_dir,
        "sidecar-variant-eip7594-one-blob-fields-7594",
        BlobTransactionSidecarVariant::Eip7594(sidecar_eip7594),
    );
    let populated_receipt = ReceiptWithBloom {
        receipt: Receipt {
            status: true.into(),
            cumulative_gas_used: 21_000,
            logs: vec![Default::default()],
        },
        logs_bloom: Default::default(),
    };
    seed(
        &corpus_dir,
        "receipt-envelope-eip1559-populated",
        ReceiptEnvelope::Eip1559(populated_receipt.clone()),
    );
    seed(
        &corpus_dir,
        "receipts-envelope-populated",
        Receipts {
            receipt_vec: vec![vec![ReceiptEnvelope::Eip1559(populated_receipt.clone())]],
        },
    );
    seed(
        &corpus_dir,
        "receipts-with-bloom-populated",
        Receipts { receipt_vec: vec![vec![populated_receipt]] },
    );
    seed(
        &corpus_dir,
        "receipt-envelope-legacy-empty",
        ReceiptEnvelope::Legacy(ReceiptWithBloom::default()),
    );
    seed(
        &corpus_dir,
        "receipt-with-bloom-ethereum-empty",
        ReceiptWithBloom::<EthereumReceipt>::default(),
    );
    let ethereum_legacy_receipt = ReceiptWithBloom::<EthereumReceipt>::default();
    seed_2718(
        &corpus_dir,
        "receipt-with-bloom-ethereum-legacy-direct",
        ethereum_legacy_receipt.clone(),
    );
    let mut tagged_ethereum_legacy_receipt = vec![0x00];
    tagged_ethereum_legacy_receipt.extend_from_slice(&ethereum_legacy_receipt.encoded_2718());
    assert!(
        ReceiptWithBloom::<EthereumReceipt>::decode_2718_exact(
            &tagged_ethereum_legacy_receipt
        )
        .is_err()
    );
    write_seed(
        &corpus_dir,
        "receipt-with-bloom-ethereum-tagged-legacy-rejected",
        &tagged_ethereum_legacy_receipt,
    );
    let mut network_tagged_ethereum_legacy_receipt = Vec::new();
    RlpHeader {
        list: false,
        payload_length: tagged_ethereum_legacy_receipt.len(),
    }
    .encode(&mut network_tagged_ethereum_legacy_receipt);
    network_tagged_ethereum_legacy_receipt
        .extend_from_slice(&tagged_ethereum_legacy_receipt);
    assert!(
        ReceiptWithBloom::<EthereumReceipt>::network_decode(
            &mut network_tagged_ethereum_legacy_receipt.as_slice()
        )
        .is_err()
    );
    write_seed(
        &corpus_dir,
        "receipt-with-bloom-ethereum-network-tagged-legacy-rejected",
        &network_tagged_ethereum_legacy_receipt,
    );
    let any_legacy_receipt =
        AnyReceiptEnvelope { inner: ReceiptWithBloom::default(), r#type: 0 };
    seed_2718(
        &corpus_dir,
        "any-receipt-envelope-legacy-empty",
        any_legacy_receipt.clone(),
    );
    let mut tagged_any_legacy_receipt = vec![0x00];
    tagged_any_legacy_receipt.extend_from_slice(&any_legacy_receipt.encoded_2718());
    assert!(AnyReceiptEnvelope::decode_2718_exact(&tagged_any_legacy_receipt).is_err());
    write_seed(
        &corpus_dir,
        "any-receipt-envelope-tagged-legacy-rejected",
        &tagged_any_legacy_receipt,
    );
    let mut network_tagged_any_legacy_receipt = Vec::new();
    RlpHeader { list: false, payload_length: tagged_any_legacy_receipt.len() }
        .encode(&mut network_tagged_any_legacy_receipt);
    network_tagged_any_legacy_receipt.extend_from_slice(&tagged_any_legacy_receipt);
    assert!(
        AnyReceiptEnvelope::network_decode(&mut network_tagged_any_legacy_receipt.as_slice())
            .is_err()
    );
    write_seed(
        &corpus_dir,
        "any-receipt-envelope-network-tagged-legacy-rejected",
        &network_tagged_any_legacy_receipt,
    );
    seed(&corpus_dir, "receipts-envelope-empty", Receipts::<ReceiptEnvelope>::default());
    seed(
        &corpus_dir,
        "receipts-with-bloom-empty",
        Receipts::<ReceiptWithBloom<Receipt>>::default(),
    );
    seed_2718(
        &corpus_dir,
        "tx-envelope-legacy-default",
        TxEnvelope::Legacy(TxLegacy::default().into_signed(signature)),
    );
    let legacy = TxEnvelope::Legacy(TxLegacy::default().into_signed(signature));
    let mut tagged_legacy = vec![0x00];
    tagged_legacy.extend_from_slice(&legacy.encoded_2718());
    assert!(
        TxEnvelope::decode_2718_exact(&tagged_legacy).is_err(),
        "an explicit type-0 prefix must not be accepted as a legacy transaction"
    );
    write_seed(&corpus_dir, "tx-envelope-tagged-legacy-rejected", &tagged_legacy);

    let mut network_tagged_legacy =
        Vec::with_capacity(RlpHeader { list: false, payload_length: tagged_legacy.len() }.length());
    RlpHeader { list: false, payload_length: tagged_legacy.len() }
        .encode(&mut network_tagged_legacy);
    network_tagged_legacy.extend_from_slice(&tagged_legacy);
    assert!(
        TxEnvelope::decode(&mut network_tagged_legacy.as_slice()).is_err(),
        "an RLP-wrapped type-0 prefix must not be accepted as a legacy network transaction"
    );
    write_seed(
        &corpus_dir,
        "tx-envelope-network-tagged-legacy-rejected",
        &network_tagged_legacy,
    );
    seed_2718(
        &corpus_dir,
        "tx-envelope-eip1559-default",
        TxEnvelope::Eip1559(TxEip1559::default().into_signed(signature)),
    );
    seed(&corpus_dir, "withdrawals-empty", Withdrawals::default());
    engine_ssz_seeds();
}
