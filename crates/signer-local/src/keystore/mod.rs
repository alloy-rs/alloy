//! Ethereum keystore management.
//!
//! Reimplements [`eth-keystore`](https://crates.io/crates/eth-keystore) functionality inline
//! using alloy primitives and direct crypto crate dependencies.

use aes::{
    cipher::{self, InnerIvInit, KeyInit, StreamCipherCore},
    Aes128,
};
use alloy_primitives::{hex, Address};
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read as _, Write as _},
    path::Path,
};
use uuid::Uuid;

const DEFAULT_CIPHER: &str = "aes-128-ctr";
const DEFAULT_KEY_SIZE: usize = 32;
const DEFAULT_IV_SIZE: usize = 16;
const DEFAULT_KDF_PARAMS_DKLEN: u8 = 32;
const DEFAULT_KDF_PARAMS_LOG_N: u8 = 13;
const DEFAULT_KDF_PARAMS_R: u32 = 8;
const DEFAULT_KDF_PARAMS_P: u32 = 1;

// ===== Error =====

/// An error thrown when interacting with the eth-keystore
#[derive(Debug, thiserror::Error)]
pub enum KeystoreError {
    /// MAC mismatch error (incorrect password or corrupted keystore).
    #[error("Mac Mismatch")]
    MacMismatch,
    /// IO error
    #[error("IO: {0}")]
    StdIo(String),
    /// serde_json error
    #[error("serde-json: {0}")]
    SerdeJson(String),
    /// scrypt invalid params error
    #[error("scrypt {0:?}")]
    ScryptInvalidParams(scrypt::errors::InvalidParams),
    /// scrypt invalid output length error
    #[error("scrypt {0:?}")]
    ScryptInvalidOutputLen(scrypt::errors::InvalidOutputLen),
    /// AES invalid key/nonce length error
    #[error(transparent)]
    AesInvalidKeyNonceLength(#[from] aes::cipher::InvalidLength),
    /// Invalid keystore parameters.
    #[error("invalid keystore parameters: {0}")]
    InvalidParams(&'static str),
}

impl From<scrypt::errors::InvalidParams> for KeystoreError {
    fn from(e: scrypt::errors::InvalidParams) -> Self {
        Self::ScryptInvalidParams(e)
    }
}

impl From<scrypt::errors::InvalidOutputLen> for KeystoreError {
    fn from(e: scrypt::errors::InvalidOutputLen) -> Self {
        Self::ScryptInvalidOutputLen(e)
    }
}

impl From<std::io::Error> for KeystoreError {
    fn from(e: std::io::Error) -> Self {
        Self::StdIo(e.to_string())
    }
}

impl From<serde_json::Error> for KeystoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e.to_string())
    }
}

// ===== Types =====

/// Ethereum keystore file representation.
#[derive(Debug, Deserialize, Serialize)]
pub struct EthKeystore {
    /// The keystore address (optional, for geth compatibility).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    /// The crypto parameters.
    pub crypto: CryptoJson,
    /// The keystore UUID.
    pub id: Uuid,
    /// The keystore version.
    pub version: u8,
}

/// Crypto parameters for the keystore.
#[derive(Debug, Deserialize, Serialize)]
pub struct CryptoJson {
    /// The cipher type.
    pub cipher: String,
    /// The cipher parameters.
    pub cipherparams: CipherparamsJson,
    /// The ciphertext.
    #[serde(serialize_with = "buffer_to_hex", deserialize_with = "hex_to_buffer")]
    pub ciphertext: Vec<u8>,
    /// The KDF type.
    pub kdf: KdfType,
    /// The KDF parameters.
    pub kdfparams: KdfparamsType,
    /// The MAC.
    #[serde(serialize_with = "buffer_to_hex", deserialize_with = "hex_to_buffer")]
    pub mac: Vec<u8>,
}

/// Cipher parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct CipherparamsJson {
    /// The initialization vector.
    #[serde(serialize_with = "buffer_to_hex", deserialize_with = "hex_to_buffer")]
    pub iv: Vec<u8>,
}

/// KDF type enum.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KdfType {
    /// PBKDF2 KDF.
    Pbkdf2,
    /// Scrypt KDF.
    Scrypt,
}

/// KDF parameters.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum KdfparamsType {
    /// PBKDF2 parameters.
    Pbkdf2 {
        /// Iteration count.
        c: u32,
        /// Derived key length.
        dklen: u8,
        /// PRF.
        prf: String,
        /// Salt.
        #[serde(serialize_with = "buffer_to_hex", deserialize_with = "hex_to_buffer")]
        salt: Vec<u8>,
    },
    /// Scrypt parameters.
    Scrypt {
        /// Derived key length.
        dklen: u8,
        /// N parameter.
        n: u32,
        /// P parameter.
        p: u32,
        /// R parameter.
        r: u32,
        /// Salt.
        #[serde(serialize_with = "buffer_to_hex", deserialize_with = "hex_to_buffer")]
        salt: Vec<u8>,
    },
}

// ===== Hex serde helpers =====

fn buffer_to_hex<T, S>(buffer: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(buffer))
}

fn hex_to_buffer<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| hex::decode(string).map_err(|err| Error::custom(err.to_string())))
}

// ===== AES-128-CTR =====

struct Aes128Ctr {
    inner: ctr::CtrCore<Aes128, ctr::flavors::Ctr128BE>,
}

impl Aes128Ctr {
    fn new(key: &[u8], iv: &[u8]) -> Result<Self, cipher::InvalidLength> {
        let cipher = Aes128::new_from_slice(key)?;
        let inner = ctr::CtrCore::inner_iv_slice_init(cipher, iv)
            .map_err(|_| cipher::InvalidLength)?;
        Ok(Self { inner })
    }

    fn apply_keystream(self, buf: &mut [u8]) {
        self.inner.apply_keystream_partial(buf.into());
    }
}

// ===== Public API =====

/// Creates a new random encrypted JSON keystore.
///
/// Returns a tuple `(secret_key_bytes, uuid_string)`.
pub(crate) fn new<P, R, S>(
    dir: P,
    rng: &mut R,
    password: S,
    name: Option<&str>,
) -> Result<(Vec<u8>, String), KeystoreError>
where
    P: AsRef<Path>,
    R: Rng + CryptoRng,
    S: AsRef<[u8]>,
{
    let mut pk = vec![0u8; DEFAULT_KEY_SIZE];
    rng.fill_bytes(pk.as_mut_slice());
    let uuid = encrypt_key(dir, rng, &pk, password, name)?;
    Ok((pk, uuid))
}

/// Decrypts an encrypted JSON keystore at the given path with the provided password.
///
/// Returns the decrypted private key bytes.
pub(crate) fn decrypt_key<P, S>(path: P, password: S) -> Result<Vec<u8>, KeystoreError>
where
    P: AsRef<Path>,
    S: AsRef<[u8]>,
{
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let keystore: EthKeystore = serde_json::from_str(&contents)?;

    // Validate the IV length.
    if keystore.crypto.cipherparams.iv.len() < 16 {
        return Err(KeystoreError::InvalidParams("iv must be at least 16 bytes"));
    }

    // Derive the key based on KDF parameters.
    let key = match keystore.crypto.kdfparams {
        KdfparamsType::Pbkdf2 { c, dklen, prf: _, ref salt } => {
            if (dklen as usize) < 32 {
                return Err(KeystoreError::InvalidParams("dklen must be at least 32"));
            }
            let mut key = vec![0u8; dklen as usize];
            pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha256>>(
                password.as_ref(),
                salt,
                c,
                key.as_mut_slice(),
            )
            .map_err(|_| KeystoreError::MacMismatch)?;
            key
        }
        KdfparamsType::Scrypt { dklen, n, p, r, ref salt } => {
            if (dklen as usize) < 32 {
                return Err(KeystoreError::InvalidParams("dklen must be at least 32"));
            }
            if !n.is_power_of_two() || n < 2 {
                return Err(KeystoreError::InvalidParams(
                    "scrypt n must be a power of two >= 2",
                ));
            }
            let mut key = vec![0u8; dklen as usize];
            let log_n = n.ilog2() as u8;
            let scrypt_params = scrypt::Params::new(log_n, r, p, dklen as usize)?;
            scrypt::scrypt(password.as_ref(), salt, &scrypt_params, key.as_mut_slice())?;
            key
        }
    };

    // MAC verification using Keccak256.
    let mut mac_input = Vec::with_capacity(16 + keystore.crypto.ciphertext.len());
    mac_input.extend_from_slice(&key[16..32]);
    mac_input.extend_from_slice(&keystore.crypto.ciphertext);
    let derived_mac = alloy_primitives::keccak256(&mac_input);

    if derived_mac.as_slice() != keystore.crypto.mac.as_slice() {
        return Err(KeystoreError::MacMismatch);
    }

    // AES-128-CTR decryption.
    let decryptor = Aes128Ctr::new(&key[..16], &keystore.crypto.cipherparams.iv[..16])?;
    let mut pk = keystore.crypto.ciphertext;
    decryptor.apply_keystream(&mut pk);
    Ok(pk)
}

/// Encrypts the given private key and stores it as an encrypted JSON keystore.
///
/// Returns the UUID of the created keystore.
pub(crate) fn encrypt_key<P, R, B, S>(
    dir: P,
    rng: &mut R,
    pk: B,
    password: S,
    name: Option<&str>,
) -> Result<String, KeystoreError>
where
    P: AsRef<Path>,
    R: Rng + CryptoRng,
    B: AsRef<[u8]>,
    S: AsRef<[u8]>,
{
    let mut salt = vec![0u8; DEFAULT_KEY_SIZE];
    rng.fill_bytes(salt.as_mut_slice());

    let mut key = vec![0u8; DEFAULT_KDF_PARAMS_DKLEN as usize];
    let scrypt_params = scrypt::Params::new(
        DEFAULT_KDF_PARAMS_LOG_N,
        DEFAULT_KDF_PARAMS_R,
        DEFAULT_KDF_PARAMS_P,
        DEFAULT_KDF_PARAMS_DKLEN as usize,
    )?;
    scrypt::scrypt(password.as_ref(), &salt, &scrypt_params, key.as_mut_slice())?;

    let mut iv = vec![0u8; DEFAULT_IV_SIZE];
    rng.fill_bytes(iv.as_mut_slice());

    let encryptor = Aes128Ctr::new(&key[..16], &iv[..16])?;
    let mut ciphertext = pk.as_ref().to_vec();
    encryptor.apply_keystream(&mut ciphertext);

    // MAC using Keccak256
    let mut mac_input = Vec::with_capacity(16 + ciphertext.len());
    mac_input.extend_from_slice(&key[16..32]);
    mac_input.extend_from_slice(&ciphertext);
    let mac = alloy_primitives::keccak256(&mac_input);

    let id = Uuid::new_v4();
    let name = name.map(|n| n.to_string()).unwrap_or_else(|| id.to_string());

    let keystore = EthKeystore {
        address: None,
        crypto: CryptoJson {
            cipher: String::from(DEFAULT_CIPHER),
            cipherparams: CipherparamsJson { iv },
            ciphertext: ciphertext.to_vec(),
            kdf: KdfType::Scrypt,
            kdfparams: KdfparamsType::Scrypt {
                dklen: DEFAULT_KDF_PARAMS_DKLEN,
                n: 2u32.pow(DEFAULT_KDF_PARAMS_LOG_N as u32),
                p: DEFAULT_KDF_PARAMS_P,
                r: DEFAULT_KDF_PARAMS_R,
                salt,
            },
            mac: mac.to_vec(),
        },
        id,
        version: 3,
    };
    let contents = serde_json::to_string(&keystore)?;

    let mut file = File::create(dir.as_ref().join(&name))?;
    file.write_all(contents.as_bytes())?;
    Ok(id.to_string())
}
