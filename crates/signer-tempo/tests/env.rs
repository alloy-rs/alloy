//! Environment-variable resolution.

use alloy_primitives::{address, Address};
use alloy_signer_tempo::{
    tempo_signer_from_env, TempoSignerError, ENV_ACCESS_KEY, ENV_PRIVATE_KEY, ENV_ROOT_ACCOUNT,
};
use serial_test::serial;
use std::ffi::OsString;

const KEY1_HEX: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
const KEY2_HEX: &str = "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a";

const fn key1_addr() -> Address {
    address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8")
}
const fn key2_addr() -> Address {
    address!("0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65")
}
const fn root_addr() -> Address {
    address!("0xa0Ee7A142d267C1f36714E4a8F75612F20a79720")
}
const KEY2_ADDR_HEX: &str = "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65";
const ROOT_HEX: &str = "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720";

struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn new(keys: &'static [&'static str]) -> Self {
        let saved = keys.iter().map(|k| (*k, std::env::var_os(k))).collect::<Vec<_>>();
        for k in keys {
            std::env::remove_var(k);
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

const ALL: &[&str] = &[ENV_PRIVATE_KEY, ENV_ACCESS_KEY, ENV_ROOT_ACCOUNT];

#[test]
#[serial]
fn no_env_returns_none() {
    let _g = EnvGuard::new(ALL);
    assert!(tempo_signer_from_env().unwrap().is_none());
}

#[test]
#[serial]
fn private_key_env_returns_direct() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_PRIVATE_KEY, KEY1_HEX);

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    assert!(lookup.is_direct());
    assert_eq!(lookup.signer().address(), key1_addr());
}

#[test]
#[serial]
fn access_key_with_root_returns_keychain() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_ACCESS_KEY, KEY2_HEX);
    std::env::set_var(ENV_ROOT_ACCOUNT, ROOT_HEX);

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    assert!(lookup.is_keychain());

    let ak = lookup.access_key().unwrap();
    assert_eq!(ak.wallet_address, root_addr());
    assert_eq!(ak.key_address, key2_addr());
    assert!(ak.key_authorization.is_none());
    assert_eq!(ak.chain_id, 0);

    assert_eq!(lookup.signer().address(), key2_addr());
    assert_eq!(lookup.from_address(), root_addr());
}

#[test]
#[serial]
fn access_key_without_root_returns_direct() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_ACCESS_KEY, KEY2_HEX);

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    assert!(lookup.is_direct());
    assert_eq!(lookup.signer().address(), key2_addr());
}

#[test]
#[serial]
fn access_key_with_root_equal_to_signer_returns_direct() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_ACCESS_KEY, KEY2_HEX);
    std::env::set_var(ENV_ROOT_ACCOUNT, KEY2_ADDR_HEX);

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    assert!(lookup.is_direct());
}

#[test]
#[serial]
fn access_key_takes_precedence_over_private_key() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_PRIVATE_KEY, KEY1_HEX);
    std::env::set_var(ENV_ACCESS_KEY, KEY2_HEX);

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    // Should use ENV_ACCESS_KEY (KEY2), not ENV_PRIVATE_KEY (KEY1).
    assert_eq!(lookup.signer().address(), key2_addr());
}

#[test]
#[serial]
fn bad_hex_in_private_key_env() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_PRIVATE_KEY, "0xnothex");

    let err = tempo_signer_from_env().unwrap_err();
    assert!(matches!(
        err,
        TempoSignerError::BadEnvHex { var } if var == ENV_PRIVATE_KEY
    ));
}

#[test]
#[serial]
fn bad_hex_in_access_key_env() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_ACCESS_KEY, "0xnothex");

    let err = tempo_signer_from_env().unwrap_err();
    assert!(matches!(
        err,
        TempoSignerError::BadEnvHex { var } if var == ENV_ACCESS_KEY
    ));
}

#[test]
#[serial]
fn bad_address_in_root_account_env() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_ACCESS_KEY, KEY2_HEX);
    std::env::set_var(ENV_ROOT_ACCOUNT, "not-an-address");

    let err = tempo_signer_from_env().unwrap_err();
    assert!(matches!(
        err,
        TempoSignerError::BadEnvAddress { var } if var == ENV_ROOT_ACCOUNT
    ));
}

#[test]
#[serial]
fn whitespace_in_env_is_trimmed() {
    let _g = EnvGuard::new(ALL);
    std::env::set_var(ENV_PRIVATE_KEY, format!("  {}\n", KEY1_HEX));

    let lookup = tempo_signer_from_env().unwrap().unwrap();
    assert_eq!(lookup.signer().address(), key1_addr());
}
