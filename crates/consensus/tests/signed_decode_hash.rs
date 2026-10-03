//! The hash cached by `Signed::eip2718_decode` and `Signed::network_decode` must be the
//! transaction hash, which for blob transactions excludes the sidecar.

use alloy_consensus::{
    transaction::RlpEcdsaDecodableTx, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEip7702, TxEnvelope, TxLegacy,
};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::{hex, Address, Signature, TxKind, B256, U256};
use std::path::PathBuf;

fn assert_decoded_hash<T: RlpEcdsaDecodableTx>(eip2718: &[u8], name: &str) {
    let signed = Signed::<T>::eip2718_decode(&mut &eip2718[..]).unwrap();
    let expected = signed.tx().tx_hash(signed.signature());
    assert_eq!(*signed.hash(), expected, "eip2718_decode: {name}");
    let envelope = TxEnvelope::decode_2718(&mut &eip2718[..]).unwrap();
    assert_eq!(*envelope.tx_hash(), expected, "envelope decode_2718: {name}");

    let mut network = Vec::new();
    signed.network_encode(&mut network);
    let signed = Signed::<T>::network_decode(&mut &network[..]).unwrap();
    assert_eq!(*signed.hash(), expected, "network_decode: {name}");
    let envelope = TxEnvelope::network_decode(&mut &network[..]).unwrap();
    assert_eq!(*envelope.tx_hash(), expected, "envelope network_decode: {name}");
}

fn assert_signed_decoded_hash<T: RlpEcdsaDecodableTx>(tx: T, name: &str) {
    let signature = Signature::new(U256::from(1), U256::from(2), true);
    let mut eip2718 = Vec::new();
    tx.eip2718_encode(&signature, &mut eip2718);
    assert_decoded_hash::<T>(&eip2718, name);
}

#[test]
fn decoded_hash_matches_tx_hash() {
    let to = Address::with_last_byte(1);
    let legacy =
        TxLegacy { nonce: 1, gas_limit: 21_000, to: TxKind::Call(to), ..Default::default() };
    assert_signed_decoded_hash(legacy.clone(), "legacy");
    assert_signed_decoded_hash(TxLegacy { chain_id: Some(1), ..legacy }, "legacy eip155");
    assert_signed_decoded_hash(
        TxEip2930 {
            chain_id: 1,
            nonce: 1,
            gas_limit: 21_000,
            to: TxKind::Call(to),
            ..Default::default()
        },
        "eip2930",
    );
    assert_signed_decoded_hash(
        TxEip1559 {
            chain_id: 1,
            nonce: 1,
            gas_limit: 21_000,
            to: TxKind::Call(to),
            ..Default::default()
        },
        "eip1559",
    );
    let eip4844 = TxEip4844 {
        chain_id: 1,
        nonce: 1,
        gas_limit: 21_000,
        to,
        blob_versioned_hashes: vec![B256::with_last_byte(1)],
        ..Default::default()
    };
    assert_signed_decoded_hash(eip4844.clone(), "eip4844");
    let variant: TxEip4844Variant = eip4844.into();
    assert_signed_decoded_hash(variant, "eip4844 variant");
    assert_signed_decoded_hash(
        TxEip7702 { chain_id: 1, nonce: 1, gas_limit: 21_000, to, ..Default::default() },
        "eip7702",
    );
}

#[test]
fn decoded_hash_excludes_sidecar() {
    for dir in ["testdata/4844rlp", "testdata/7594rlp"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir);
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let raw = hex::decode(std::fs::read_to_string(&path).unwrap().trim()).unwrap();
            let name = path.display().to_string();
            assert_decoded_hash::<TxEip4844WithSidecar>(&raw, &name);
            assert_decoded_hash::<TxEip4844Variant>(&raw, &name);
        }
    }
}
