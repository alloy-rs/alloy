use crate::{
    entry::{EntrySummary, KeyType, RawKeyEntry},
    error::TempoSignerError,
    lookup::{TempoAccessKey, TempoLookup},
};
use alloy_primitives::{hex, Address, Bytes, B256};
use alloy_signer_local::PrivateKeySigner;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const KEYS_FILE_NAME: &str = "keys.toml";
const TEMPO_HOME_ENV: &str = "TEMPO_HOME";
const TEMPO_HOME_DEFAULT_DIR: &str = ".tempo";

/// Default keystore path: `$TEMPO_HOME/wallet/keys.toml`, falling back to
/// `~/.tempo/wallet/keys.toml`. Mirrors the Tempo CLI exactly.
pub fn default_keys_path() -> Result<PathBuf, TempoSignerError> {
    let base = match std::env::var_os(TEMPO_HOME_ENV) {
        Some(home) => PathBuf::from(home),
        None => home_dir_or_err()?.join(TEMPO_HOME_DEFAULT_DIR),
    };
    Ok(base.join("wallet").join(KEYS_FILE_NAME))
}

#[cfg(not(target_family = "wasm"))]
fn home_dir_or_err() -> Result<PathBuf, TempoSignerError> {
    dirs::home_dir().ok_or(TempoSignerError::NoHome)
}

#[cfg(target_family = "wasm")]
fn home_dir_or_err() -> Result<PathBuf, TempoSignerError> {
    Err(TempoSignerError::NoHome)
}

#[derive(Deserialize)]
struct RawKeystore {
    #[serde(default)]
    keys: Vec<RawKeyEntry>,
}

/// Parsed Tempo keystore, ready to be looked up.
pub struct TempoKeystore {
    entries: Vec<RawKeyEntry>,
    path: PathBuf,
}

impl std::fmt::Debug for TempoKeystore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TempoKeystore")
            .field("path", &self.path)
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl TempoKeystore {
    /// Load the keystore from the [`default_keys_path`].
    pub fn load() -> Result<Self, TempoSignerError> {
        Self::load_from(default_keys_path()?)
    }

    /// Load the keystore from an explicit path.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, TempoSignerError> {
        let path = path.as_ref().to_path_buf();
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(TempoSignerError::NotFound { path });
            }
            Err(e) => return Err(TempoSignerError::Io(e)),
        };

        let entries = parse_keystore(&contents, &path)?;
        Ok(Self { entries, path })
    }

    /// Returns the path the keystore was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of entries in the keystore.
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the keystore has zero entries.
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over redacted summaries of all entries (no secrets).
    pub fn iter(&self) -> impl Iterator<Item = EntrySummary<'_>> {
        self.entries.iter().map(|e| EntrySummary {
            wallet_type: e.wallet_type,
            key_type: e.key_type,
            wallet_address: e.wallet_address,
            key_address: e.key_address,
            chain_id: e.chain_id,
            expiry: e.expiry,
            limits: &e.limits,
            has_key: e.key.is_some(),
            has_key_authorization: e.key_authorization.is_some(),
        })
    }

    /// Find a usable key by wallet/EOA address (`from`).
    pub fn find_by_from(&self, from: Address) -> Result<TempoLookup, TempoSignerError> {
        let candidates: Vec<&RawKeyEntry> =
            self.entries.iter().filter(|e| e.wallet_address == from).collect();
        select_one(candidates, from)
    }

    /// Find a usable key by access-key address.
    pub fn find_by_key(&self, key_address: Address) -> Result<TempoLookup, TempoSignerError> {
        let candidates: Vec<&RawKeyEntry> =
            self.entries.iter().filter(|e| e.key_address == Some(key_address)).collect();
        select_one(candidates, key_address)
    }
}

fn select_one(
    candidates: Vec<&RawKeyEntry>,
    from: Address,
) -> Result<TempoLookup, TempoSignerError> {
    // Keep only secp256k1 entries with a private key; drop expired ones.
    let mut saw_unsupported: Option<KeyType> = None;
    let mut saw_expired: Option<u64> = None;
    let now = current_unix_seconds();

    let mut usable: Vec<&RawKeyEntry> = Vec::new();
    for c in candidates {
        if c.key_type != KeyType::Secp256k1 || c.key.is_none() {
            saw_unsupported.get_or_insert(c.key_type);
            continue;
        }
        if let Some(exp) = c.expiry {
            if exp <= now {
                saw_expired = Some(exp);
                continue;
            }
        }
        usable.push(c);
    }

    match usable.len() {
        1 => entry_to_lookup(usable[0]),
        0 => {
            if let Some(exp) = saw_expired {
                Err(TempoSignerError::Expired { expiry: exp })
            } else if let Some(kind) = saw_unsupported {
                Err(TempoSignerError::UnsupportedKeyType { kind })
            } else {
                Err(TempoSignerError::NoMatch { from })
            }
        }
        _ => Err(TempoSignerError::Ambiguous { from }),
    }
}

fn entry_to_lookup(e: &RawKeyEntry) -> Result<TempoLookup, TempoSignerError> {
    let key_hex = e.key.as_deref().ok_or(TempoSignerError::BadKey)?;
    let signer = parse_signer(key_hex)?;
    let signer_addr = signer.address();

    // Trust the derived address over the file's `key_address` field.
    if signer_addr == e.wallet_address {
        return Ok(TempoLookup::Direct(signer));
    }

    let key_address = e.key_address.unwrap_or(signer_addr);

    let key_authorization = match &e.key_authorization {
        Some(s) => Some(parse_hex_bytes(s, "key_authorization")?),
        None => None,
    };

    Ok(TempoLookup::Keychain(
        signer,
        TempoAccessKey {
            wallet_address: e.wallet_address,
            key_address,
            key_authorization,
            chain_id: e.chain_id,
            expiry: e.expiry,
        },
    ))
}

pub(crate) fn parse_signer(hex_str: &str) -> Result<PrivateKeySigner, TempoSignerError> {
    let bytes = parse_hex32(hex_str, "key")?;
    let b256 = B256::from(bytes);
    PrivateKeySigner::from_bytes(&b256).map_err(|_| TempoSignerError::BadKey)
}

fn parse_hex32(s: &str, field: &'static str) -> Result<[u8; 32], TempoSignerError> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let mut out = [0u8; 32];
    hex::decode_to_slice(s, &mut out).map_err(|_| TempoSignerError::BadHex { field })?;
    Ok(out)
}

fn parse_hex_bytes(s: &str, field: &'static str) -> Result<Bytes, TempoSignerError> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let v = hex::decode(s).map_err(|_| TempoSignerError::BadHex { field })?;
    Ok(Bytes::from(v))
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_keystore(contents: &str, path: &Path) -> Result<Vec<RawKeyEntry>, TempoSignerError> {
    match toml::from_str::<RawKeystore>(contents) {
        Ok(ks) => Ok(ks.keys),
        Err(strict_err) => {
            // Salvage: parse each `[[keys]]` entry independently and skip malformed ones.
            let value: toml::Value = match toml::from_str(contents) {
                Ok(v) => v,
                Err(_) => {
                    return Err(TempoSignerError::BadToml {
                        path: path.to_path_buf(),
                        source: strict_err,
                    })
                }
            };
            let arr = match value.get("keys") {
                Some(toml::Value::Array(arr)) => arr.clone(),
                _ => {
                    return Err(TempoSignerError::BadToml {
                        path: path.to_path_buf(),
                        source: strict_err,
                    })
                }
            };
            let mut entries = Vec::new();
            for (i, v) in arr.into_iter().enumerate() {
                match v.try_into::<RawKeyEntry>() {
                    Ok(e) => entries.push(e),
                    Err(err) => {
                        tracing::warn!(
                            path = %path.display(),
                            index = i,
                            error = %err,
                            "skipping malformed Tempo keystore entry",
                        );
                    }
                }
            }
            Ok(entries)
        }
    }
}
