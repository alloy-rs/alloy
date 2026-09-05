//! Cross-crate regressions for frame transactions and conventional Ethereum transactions.

use alloy_consensus::{
    transaction::{PooledTransaction, Recovered, RlpEcdsaEncodableTx, TxHashRef},
    BlockBody, ReceiptEnvelope, Signed, Transaction, TxEip8141, TxEnvelope, TxType,
    TypedTransaction,
};
use alloy_eips::{
    eip7594::BlobTransactionSidecarEip7594,
    eip8141::{Frame, FrameReceiptPayload, TransactionFees},
    Decodable2718, Encodable2718,
};
use alloy_network::{AnyReceiptEnvelope, AnyTxEnvelope, Ethereum, NetworkTransactionBuilder};
use alloy_primitives::{Address, Bloom, Sealable, Signature, B256, U256};
use alloy_rpc_types_eth::TransactionRequest;

fn frame_tx() -> TxEip8141 {
    TxEip8141 {
        chain_id: 1,
        sender: Address::repeat_byte(1),
        frames: vec![Frame::default()],
        fees: TransactionFees {
            max_fee_per_gas: U256::from(100),
            max_priority_fee_per_gas: U256::from(1),
            max_fee_per_blob_gas: U256::ZERO,
        },
        ..Default::default()
    }
}

fn signature() -> Signature {
    Signature::new(U256::from(1), U256::from(1), false)
}

#[test]
fn frame_blob_hashes_are_included_in_block_order() {
    let mut first = frame_tx();
    first.blob_versioned_hashes = vec![B256::repeat_byte(1)];
    let mut second = frame_tx();
    second.blob_versioned_hashes = vec![B256::repeat_byte(2), B256::repeat_byte(3)];
    let body = BlockBody::<TxEnvelope> {
        transactions: vec![
            TxEnvelope::Eip8141(first.seal_slow()),
            TxEnvelope::Eip8141(second.seal_slow()),
        ],
        ..Default::default()
    };
    assert_eq!(
        body.blob_versioned_hashes_iter().copied().collect::<Vec<_>>(),
        vec![B256::repeat_byte(1), B256::repeat_byte(2), B256::repeat_byte(3)]
    );
}

#[test]
fn typed_frame_and_older_encodings_have_exact_lengths() {
    let txs = [
        TypedTransaction::Eip8141(frame_tx()),
        TypedTransaction::Legacy(Default::default()),
        TypedTransaction::Eip2930(Default::default()),
        TypedTransaction::Eip1559(Default::default()),
        TypedTransaction::Eip4844(alloy_consensus::TxEip4844::default().into()),
        TypedTransaction::Eip7702(Default::default()),
    ];
    for tx in txs {
        let sig = signature();
        let mut encoded = Vec::new();
        tx.rlp_encode_signed(&sig, &mut encoded);
        assert_eq!(encoded.len(), tx.rlp_header_signed(&sig).length_with_payload());
        encoded.clear();
        tx.eip2718_encode(&sig, &mut encoded);
        assert_eq!(encoded.len(), tx.eip2718_encoded_length(&sig));
        encoded.clear();
        tx.network_encode(&sig, &mut encoded);
        assert_eq!(encoded.len(), tx.network_encoded_length(&sig));
        let signed = Signed::new_unhashed(tx, sig);
        encoded.clear();
        Encodable2718::network_encode(&signed, &mut encoded);
        let mut input = encoded.as_slice();
        TxEnvelope::network_decode(&mut input).unwrap();
        assert!(input.is_empty());
    }
}

#[test]
fn frame_only_properties_are_fallible_not_panicking() {
    let mut envelope = TxEnvelope::Eip8141(frame_tx().seal_slow());
    assert!(envelope.signature().is_none());
    assert!(envelope.input_mut().is_none());
    assert!(envelope.into_signed().is_err());
    let receipt: ReceiptEnvelope = ReceiptEnvelope::Eip8141(FrameReceiptPayload::default().into());
    assert_eq!(receipt.logs_bloom(), Bloom::ZERO);
    assert!(receipt.into_receipt().is_err());
    assert!(ReceiptEnvelope::from_typed(
        TxType::Eip8141,
        alloy_consensus::Receipt::<alloy_primitives::Log>::default()
    )
    .is_err());
}

#[test]
fn fee_edits_override_converted_values() {
    let mut request: TransactionRequest = frame_tx().into();
    request.max_fee_per_gas = Some(200);
    assert_eq!(request.build_8141().unwrap().fees.max_fee_per_gas, U256::from(200));
}

#[test]
fn wide_fees_survive_json_and_rpc_wrappers() {
    let mut tx = frame_tx();
    tx.fees.max_fee_per_gas = U256::from(1) << 128;
    let request: TransactionRequest = tx.clone().into();
    assert!(request.max_fee_per_gas.is_none());
    let decoded: TransactionRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
    assert_eq!(decoded.build_8141().unwrap(), tx);
    let rpc = alloy_rpc_types_eth::Transaction {
        inner: Recovered::new_unchecked(tx.clone(), tx.sender),
        ..Default::default()
    };
    assert_eq!(rpc.max_fee_per_gas_u256(), tx.fees.max_fee_per_gas);
    let mut edited = request;
    edited.max_fee_per_gas = Some(200);
    assert_eq!(edited.build_8141().unwrap().fees.max_fee_per_gas, U256::from(200));
}

#[test]
fn generic_conversion_preserves_frames_through_envelopes() {
    let mut tx = frame_tx();
    tx.frames[0].data = vec![1, 0, 2].into();
    tx.signatures = vec![alloy_eips::eip8141::FrameSignature {
        signature: vec![3, 4, 5].into(),
        ..Default::default()
    }];
    let envelope = TxEnvelope::Eip8141(tx.clone().seal_slow());
    let request = TransactionRequest::from_transaction(AnyTxEnvelope::Ethereum(envelope));
    assert!(request.from.is_none());
    assert_eq!(request.from(tx.sender).build_8141().unwrap(), tx);
}

#[test]
fn completion_and_build_move_frame_storage() {
    let request: TransactionRequest = frame_tx().into();
    let frames = request.frames.as_ref().unwrap().as_ptr();
    request.complete_8141().unwrap();
    assert_eq!(request.build_8141().unwrap().frames.as_ptr(), frames);
}

#[test]
fn frame_receipts_roundtrip_through_any_network() {
    let receipt: ReceiptEnvelope = ReceiptEnvelope::Eip8141(
        FrameReceiptPayload {
            cumulative_gas_used: 12475,
            payer: Address::repeat_byte(1),
            frame_receipts: vec![Default::default()],
        }
        .into(),
    );
    let encoded = receipt.encoded_2718();
    let decoded = AnyReceiptEnvelope::decode_2718_exact(&encoded).unwrap();
    assert_eq!(decoded.encoded_2718(), encoded);
    let json = serde_json::to_value(&receipt).unwrap();
    let decoded: AnyReceiptEnvelope = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(decoded.encoded_2718(), encoded);
    let mut invalid = json;
    invalid.as_object_mut().unwrap().remove("payer");
    assert!(serde_json::from_value::<AnyReceiptEnvelope>(invalid).is_err());
}

#[test]
fn any_network_preserves_unknown_conventional_receipts() {
    let receipt = AnyReceiptEnvelope::Other {
        inner: alloy_consensus::Receipt::default().with_bloom(),
        r#type: 0x7f,
    };
    let bytes = receipt.encoded_2718();
    assert_eq!(AnyReceiptEnvelope::decode_2718_exact(&bytes).unwrap(), receipt);
    let json = serde_json::to_value(&receipt).unwrap();
    assert_eq!(serde_json::from_value::<AnyReceiptEnvelope>(json).unwrap(), receipt);
}

#[test]
fn pooled_frame_blobs_require_sidecars() {
    let mut tx = frame_tx();
    tx.blob_versioned_hashes = vec![B256::repeat_byte(1)];
    tx.fees.max_fee_per_blob_gas = U256::from(1);
    assert!(PooledTransaction::try_from(tx.clone()).is_err());
    assert!(PooledTransaction::decode_2718_exact(&tx.encoded_2718()).is_err());
    let canonical: alloy_consensus::EthereumTxEnvelope<alloy_consensus::TxEip4844> =
        alloy_consensus::EthereumTxEnvelope::Eip8141(tx.clone().seal_slow());
    assert!(canonical.clone().try_into_pooled::<BlobTransactionSidecarEip7594>().is_err());
    assert!(PooledTransaction::try_from(canonical).is_err());
    let envelope = TxEnvelope::Eip8141(tx.seal_slow());
    assert!(envelope.clone().try_into_pooled().is_err());
    assert!(serde_json::from_value::<PooledTransaction>(serde_json::to_value(envelope).unwrap())
        .is_err());
}

#[test]
fn provider_build_path_retains_frame_sidecar() {
    let sidecar = BlobTransactionSidecarEip7594 {
        commitments: vec![Default::default()],
        ..Default::default()
    };
    let expected_hashes: Vec<_> = sidecar.versioned_hashes().collect();
    let mut request: TransactionRequest = frame_tx().into();
    request.max_fee_per_blob_gas = Some(1);
    request.blob_versioned_hashes = None;
    request.sidecar = Some(sidecar.clone().into());
    assert!(request.clone().build_typed_tx().is_err());
    assert!(<TransactionRequest as NetworkTransactionBuilder<Ethereum>>::build_unsigned(
        request.clone()
    )
    .is_err());
    let (canonical, encoded) = request.build_presigned_with_sidecar().unwrap();
    assert_eq!(canonical.blob_versioned_hashes().unwrap(), expected_hashes);
    let mut decoded = PooledTransaction::decode_2718_exact(&encoded).unwrap();
    assert_eq!(decoded.as_eip8141().unwrap().sidecar(), Some(&sidecar));
    assert_eq!(decoded.tx_hash(), canonical.tx_hash());
    let cached = decoded.gas_limit();
    assert_eq!(cached, decoded.as_eip8141().unwrap().tx().gas_limit());
    let json = serde_json::to_value(&decoded).unwrap();
    assert_eq!(serde_json::from_value::<PooledTransaction>(json).unwrap(), decoded);
    decoded.clear_eip7594_blobs();
    assert_eq!(decoded.gas_limit(), cached);
    assert_eq!(decoded.tx_hash(), canonical.tx_hash());
}

#[test]
fn frame_receipt_memory_includes_flattened_log_storage() {
    use alloy_consensus::InMemorySize;
    use alloy_eips::eip8141::FrameReceipt;
    use alloy_primitives::Log;
    let frame: FrameReceipt<Log> =
        FrameReceipt { logs: vec![Log::default()], ..Default::default() };
    let per_frame = core::mem::size_of_val(&frame) + frame.logs[0].size();
    let flattened = frame.logs[0].size();
    let receipt: ReceiptEnvelope = ReceiptEnvelope::Eip8141(
        FrameReceiptPayload { frame_receipts: vec![frame], ..Default::default() }.into(),
    );
    assert_eq!(receipt.size(), core::mem::size_of_val(&receipt) + per_frame + flattened);
}

#[test]
fn mismatched_sidecar_hashes_return_the_original_request() {
    let mut request: TransactionRequest = frame_tx().into();
    request.sidecar = Some(
        BlobTransactionSidecarEip7594 {
            commitments: vec![Default::default()],
            ..Default::default()
        }
        .into(),
    );
    request.max_fee_per_blob_gas = Some(1);
    request.blob_versioned_hashes = Some(vec![B256::repeat_byte(1)]);
    let original = request.clone();
    assert_eq!(request.build_8141_with_sidecar().unwrap_err().into_value(), original);
}

#[test]
fn older_type_inference_is_unchanged() {
    let request = TransactionRequest {
        transaction_type: Some(0),
        max_fee_per_gas: Some(2),
        max_priority_fee_per_gas: Some(1),
        ..Default::default()
    };
    assert_eq!(request.preferred_type(), TxType::Eip1559);
}

#[test]
fn generic_wallet_support_remains_available() {
    fn check<N: alloy_network::Network>()
    where
        N::TxEnvelope: From<Signed<N::UnsignedTx>>,
        N::UnsignedTx: alloy_consensus::SignableTransaction<Signature>,
    {
        fn wallet<N: alloy_network::Network, W: alloy_network::NetworkWallet<N>>() {}
        wallet::<N, alloy_network::EthereumWallet>();
    }
    check::<Ethereum>();
    check::<alloy_network::AnyNetwork>();
}
