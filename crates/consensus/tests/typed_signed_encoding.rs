//! `Signed<EthereumTypedTransaction>` must encode exactly like the envelope it was converted from.

use alloy_consensus::{
    BlobTransactionSidecar, BlobTransactionSidecarVariant, EthereumTxEnvelope, SignableTransaction,
    Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar, TxEip7702,
    TxEnvelope, TxLegacy,
};
use alloy_eips::{
    eip2718::Encodable2718,
    eip4844::{Blob, Bytes48},
};
use alloy_primitives::{Address, Signature, TxKind, B256, U256};

const TO: Address = Address::with_last_byte(1);

const SIGNATURE: Signature =
    Signature::new(U256::from_be_bytes([0x11; 32]), U256::from_be_bytes([0x22; 32]), true);

fn assert_signed_encoding(envelope: TxEnvelope) {
    let eip2718 = envelope.encoded_2718();
    let mut network = Vec::new();
    envelope.network_encode(&mut network);
    let mut rlp = Vec::new();
    match &envelope {
        EthereumTxEnvelope::Legacy(tx) => tx.rlp_encode(&mut rlp),
        EthereumTxEnvelope::Eip2930(tx) => tx.rlp_encode(&mut rlp),
        EthereumTxEnvelope::Eip1559(tx) => tx.rlp_encode(&mut rlp),
        EthereumTxEnvelope::Eip4844(tx) => tx.rlp_encode(&mut rlp),
        EthereumTxEnvelope::Eip7702(tx) => tx.rlp_encode(&mut rlp),
    }
    assert_eq!(envelope.encode_2718_len(), eip2718.len());
    assert_eq!(envelope.network_len(), network.len());

    let signed = envelope.into_signed();
    let mut trait_network = Vec::new();
    Encodable2718::network_encode(&signed, &mut trait_network);
    let mut signed_network = Vec::new();
    signed.network_encode(&mut signed_network);
    let mut signed_rlp = Vec::new();
    signed.rlp_encode(&mut signed_rlp);

    let mismatches: Vec<_> = [
        ("encoded_2718", signed.encoded_2718() == eip2718),
        ("encode_2718_len", signed.encode_2718_len() == eip2718.len()),
        ("Encodable2718::network_encode", trait_network == network),
        ("network_len", signed.network_len() == network.len()),
        ("Signed::network_encode", signed_network == network),
        ("network_encoded_length", signed.network_encoded_length() == network.len()),
        ("rlp_encode", signed_rlp == rlp),
        ("rlp_encoded_length", signed.rlp_encoded_length() == rlp.len()),
    ]
    .into_iter()
    .filter_map(|(name, ok)| (!ok).then_some(name))
    .collect();
    assert!(mismatches.is_empty(), "mismatched: {mismatches:?}");
}

fn legacy() -> TxLegacy {
    TxLegacy { nonce: 1, gas_limit: 21_000, to: TxKind::Call(TO), ..Default::default() }
}

fn eip4844() -> TxEip4844 {
    TxEip4844 {
        chain_id: 1,
        nonce: 1,
        gas_limit: 21_000,
        to: TO,
        blob_versioned_hashes: vec![B256::with_last_byte(1)],
        // makes the signed RLP list header one byte longer than the unsigned one
        input: vec![0; 128].into(),
        ..Default::default()
    }
}

fn eip4844_envelope(tx: impl Into<TxEip4844Variant>) -> TxEnvelope {
    Signed::new_unhashed(tx.into(), SIGNATURE).into()
}

#[test]
fn legacy_without_chain_id() {
    assert_signed_encoding(legacy().into_signed(SIGNATURE).into());
}

#[test]
fn legacy_with_chain_id() {
    let tx = TxLegacy { chain_id: Some(1337), ..legacy() };
    assert_signed_encoding(tx.into_signed(SIGNATURE).into());
}

#[test]
fn eip2930() {
    let tx = TxEip2930 {
        chain_id: 1,
        nonce: 1,
        gas_limit: 21_000,
        to: TxKind::Call(TO),
        ..Default::default()
    };
    assert_signed_encoding(tx.into_signed(SIGNATURE).into());
}

#[test]
fn eip1559() {
    let tx = TxEip1559 {
        chain_id: 1,
        nonce: 1,
        gas_limit: 21_000,
        to: TxKind::Call(TO),
        ..Default::default()
    };
    assert_signed_encoding(tx.into_signed(SIGNATURE).into());
}

#[test]
fn eip4844_without_sidecar() {
    assert_signed_encoding(eip4844_envelope(eip4844()));
}

#[test]
fn eip4844_with_sidecar() {
    let sidecar = BlobTransactionSidecarVariant::Eip4844(BlobTransactionSidecar::new(
        vec![Blob::default()],
        vec![Bytes48::default()],
        vec![Bytes48::default()],
    ));
    assert_signed_encoding(eip4844_envelope(TxEip4844WithSidecar::from_tx_and_sidecar(
        eip4844(),
        sidecar,
    )));
}

#[test]
fn eip7702() {
    let tx = TxEip7702 { chain_id: 1, nonce: 1, gas_limit: 21_000, to: TO, ..Default::default() };
    assert_signed_encoding(tx.into_signed(SIGNATURE).into());
}
