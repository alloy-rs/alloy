use alloy_primitives::Address;
use serde::Deserialize;

/// On-chain wallet type recorded for an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum WalletType {
    /// EOA: the on-disk key is the wallet account.
    Local,
    /// Smart wallet: the on-disk key is an access key authorized by a passkey.
    Passkey,
    /// Unrecognized variant.
    #[serde(other)]
    Unknown,
}

/// Cryptographic key type recorded for an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum KeyType {
    /// secp256k1 ECDSA. The only type this crate can materialize.
    Secp256k1,
    /// p256 (NIST P-256) ECDSA.
    P256,
    /// WebAuthn passkey signature.
    #[serde(rename = "webauthn")]
    WebAuthn,
    /// Unrecognized variant.
    #[serde(other)]
    Unknown,
}

/// Per-token spending limit attached to an access key.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenLimit {
    /// ERC-20 token contract address.
    pub currency: Address,
    /// Spending limit as a decimal-string integer (kept as `String` to avoid precision loss).
    pub limit: String,
}

/// Internal TOML schema. Not part of the public API.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawKeyEntry {
    pub(crate) wallet_type: WalletType,
    pub(crate) wallet_address: Address,
    pub(crate) chain_id: u64,
    pub(crate) key_type: KeyType,
    pub(crate) key_address: Option<Address>,
    /// 0x-hex secp256k1 private key.
    pub(crate) key: Option<String>,
    /// 0x-hex RLP-encoded `SignedKeyAuthorization`.
    pub(crate) key_authorization: Option<String>,
    pub(crate) expiry: Option<u64>,
    #[serde(default)]
    pub(crate) limits: Vec<TokenLimit>,
}

/// Redacted summary of a single keystore entry. Contains no secret material.
#[derive(Debug, Clone)]
pub struct EntrySummary<'a> {
    /// Wallet type.
    pub wallet_type: WalletType,
    /// Key type.
    pub key_type: KeyType,
    /// Smart-wallet/EOA address (the `from` for transactions).
    pub wallet_address: Address,
    /// Address derived from the on-disk key, if recorded.
    pub key_address: Option<Address>,
    /// Chain ID this entry is bound to.
    pub chain_id: u64,
    /// Expiry as a unix timestamp, if any.
    pub expiry: Option<u64>,
    /// Per-token spending limits.
    pub limits: &'a [TokenLimit],
    /// Whether the entry has a usable private key.
    pub has_key: bool,
    /// Whether the entry has a `key_authorization` field.
    pub has_key_authorization: bool,
}
