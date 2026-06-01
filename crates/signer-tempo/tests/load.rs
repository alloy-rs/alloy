//! Loading and parsing tests for `TempoKeystore`.

use alloy_signer_tempo::{default_keys_path, TempoKeystore, TempoSignerError};
use serial_test::serial;
use std::{fs, path::PathBuf};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(FIXTURE_DIR).join(name)
}

#[test]
fn load_strict_parses_all_entries() {
    let store = TempoKeystore::load_from(fixture("keys.toml")).unwrap();
    assert_eq!(store.len(), 4);
    assert!(!store.is_empty());
    assert_eq!(store.path(), fixture("keys.toml"));
}

#[test]
fn iter_returns_redacted_summaries() {
    let store = TempoKeystore::load_from(fixture("keys.toml")).unwrap();
    let summaries: Vec<_> = store.iter().collect();
    assert_eq!(summaries.len(), 4);

    // First entry is the Direct EOA — secp256k1, has a key, no expiry, no auth.
    let direct = &summaries[0];
    assert!(direct.has_key);
    assert!(!direct.has_key_authorization);
    assert!(direct.expiry.is_none());

    // Second entry is the Keychain entry — has both key and authorization.
    let keychain = &summaries[1];
    assert!(keychain.has_key);
    assert!(keychain.has_key_authorization);
    assert_eq!(keychain.expiry, Some(4102444800));
    assert_eq!(keychain.limits.len(), 1);
}

#[test]
fn load_salvage_skips_malformed_entries() {
    let store = TempoKeystore::load_from(fixture("keys_salvage.toml")).unwrap();
    assert_eq!(store.len(), 1, "only the good entry should survive");
}

#[test]
fn load_missing_file_returns_not_found() {
    let err = TempoKeystore::load_from(fixture("does_not_exist.toml")).unwrap_err();
    assert!(matches!(err, TempoSignerError::NotFound { .. }));
}

#[test]
fn load_empty_keys_array_yields_empty_keystore() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keys.toml");
    fs::write(&path, "# empty\n").unwrap();
    let store = TempoKeystore::load_from(&path).unwrap();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn load_completely_invalid_toml_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keys.toml");
    fs::write(&path, "this is :: not = toml{[").unwrap();
    let err = TempoKeystore::load_from(&path).unwrap_err();
    assert!(matches!(err, TempoSignerError::BadToml { .. }));
}

#[test]
#[serial]
fn default_keys_path_uses_tempo_home_when_set() {
    let prev = std::env::var_os("TEMPO_HOME");
    std::env::set_var("TEMPO_HOME", "/custom/tempo/dir");
    let path = default_keys_path().unwrap();
    assert_eq!(path, PathBuf::from("/custom/tempo/dir/wallet/keys.toml"));
    restore_env("TEMPO_HOME", prev);
}

#[test]
#[serial]
fn default_keys_path_falls_back_to_home_dir() {
    let prev = std::env::var_os("TEMPO_HOME");
    std::env::remove_var("TEMPO_HOME");
    let path = default_keys_path().expect("expected a home dir on this host");
    assert!(
        path.ends_with(".tempo/wallet/keys.toml"),
        "unexpected default path: {}",
        path.display()
    );
    restore_env("TEMPO_HOME", prev);
}

fn restore_env(key: &str, prev: Option<std::ffi::OsString>) {
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}
