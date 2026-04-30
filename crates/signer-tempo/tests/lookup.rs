//! `find_by_from` / `find_by_key` semantics.

use alloy_primitives::{address, Bytes};
use alloy_signer_tempo::{KeyType, TempoKeystore, TempoSignerError};
use std::path::PathBuf;

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(FIXTURE_DIR).join(name)
}

fn store() -> TempoKeystore {
    TempoKeystore::load_from(fixture("keys.toml")).unwrap()
}

#[test]
fn find_by_from_returns_direct_for_eoa() {
    let from = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
    let lookup = store().find_by_from(from).unwrap();

    assert!(lookup.is_direct());
    assert!(!lookup.is_keychain());
    assert!(lookup.access_key().is_none());
    assert_eq!(lookup.from_address(), from);
    assert_eq!(lookup.signer().address(), from);

    // into_signer consumes the lookup.
    let signer = lookup.into_signer();
    assert_eq!(signer.address(), from);
}

#[test]
fn find_by_from_returns_keychain_for_smart_wallet() {
    let wallet = address!("0xa0Ee7A142d267C1f36714E4a8F75612F20a79720");
    let key_addr = address!("0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65");

    let lookup = store().find_by_from(wallet).unwrap();
    assert!(lookup.is_keychain());

    let ak = lookup.access_key().expect("keychain has access-key metadata");
    assert_eq!(ak.wallet_address, wallet);
    assert_eq!(ak.key_address, key_addr);
    assert_eq!(ak.chain_id, 4217);
    assert_eq!(ak.expiry, Some(4102444800));
    assert_eq!(ak.key_authorization, Some(Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef])));

    // The from-address for transactions is the wallet, NOT the signer.
    assert_eq!(lookup.from_address(), wallet);
    assert_eq!(lookup.signer().address(), key_addr);
}

#[test]
fn find_by_from_no_match() {
    let unknown = address!("0x000000000000000000000000000000000000ffff");
    let err = store().find_by_from(unknown).unwrap_err();
    assert!(matches!(err, TempoSignerError::NoMatch { from } if from == unknown));
}

#[test]
fn find_by_from_unsupported_key_type() {
    // The p256 entry has wallet_address 0x...aaaa.
    let from = address!("0x000000000000000000000000000000000000aaaa");
    let err = store().find_by_from(from).unwrap_err();
    assert!(matches!(err, TempoSignerError::UnsupportedKeyType { kind: KeyType::P256 }));
}

#[test]
fn find_by_from_expired() {
    // The expired entry has wallet_address 0x...cccc.
    let from = address!("0x000000000000000000000000000000000000cccc");
    let err = store().find_by_from(from).unwrap_err();
    assert!(matches!(err, TempoSignerError::Expired { expiry: 1000000000 }));
}

#[test]
fn find_by_from_ambiguous() {
    let store = TempoKeystore::load_from(fixture("keys_ambiguous.toml")).unwrap();
    let wallet = address!("0x000000000000000000000000000000000000dddd");
    let err = store.find_by_from(wallet).unwrap_err();
    assert!(matches!(err, TempoSignerError::Ambiguous { from } if from == wallet));
}

#[test]
fn find_by_key_finds_direct_entry() {
    // For the Direct EOA entry, key_address == wallet_address.
    let key = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
    let lookup = store().find_by_key(key).unwrap();
    assert!(lookup.is_direct());
}

#[test]
fn find_by_key_finds_keychain_entry() {
    // For the Keychain entry, key_address differs from wallet_address.
    let key = address!("0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65");
    let lookup = store().find_by_key(key).unwrap();
    assert!(lookup.is_keychain());
    assert_eq!(lookup.signer().address(), key);
}

#[test]
fn find_by_key_no_match() {
    let key = address!("0x000000000000000000000000000000000000ffff");
    let err = store().find_by_key(key).unwrap_err();
    assert!(matches!(err, TempoSignerError::NoMatch { .. }));
}

#[test]
fn debug_redacts_secrets() {
    let from = address!("0xa0Ee7A142d267C1f36714E4a8F75612F20a79720");
    let lookup = store().find_by_from(from).unwrap();
    let dbg = format!("{:?}", lookup);
    // The Debug representation must NOT contain the raw private-key hex.
    assert!(!dbg.contains("47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"));

    // The TempoAccessKey Debug must report `has_key_authorization` boolean,
    // not the raw bytes.
    let ak_dbg = format!("{:?}", lookup.access_key().unwrap());
    assert!(ak_dbg.contains("has_key_authorization: true"));
    assert!(!ak_dbg.contains("deadbeef"));
}

#[test]
fn into_signer_works_for_keychain() {
    let from = address!("0xa0Ee7A142d267C1f36714E4a8F75612F20a79720");
    let lookup = store().find_by_from(from).unwrap();
    let signer = lookup.into_signer();
    // Signer is the access key, not the wallet.
    assert_eq!(signer.address(), address!("0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65"));
}

#[test]
fn entries_with_missing_key_treated_as_unsupported() {
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keys.toml");
    fs::write(
        &path,
        r#"[[keys]]
wallet_type = "passkey"
wallet_address = "0x000000000000000000000000000000000000eeee"
chain_id = 4217
key_type = "secp256k1"
key_address = "0x000000000000000000000000000000000000eeee"
"#,
    )
    .unwrap();
    let store = TempoKeystore::load_from(&path).unwrap();
    let err =
        store.find_by_from(address!("0x000000000000000000000000000000000000eeee")).unwrap_err();
    assert!(matches!(err, TempoSignerError::UnsupportedKeyType { .. }));
}
