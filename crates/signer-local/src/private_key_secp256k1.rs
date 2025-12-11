//! [`secp256k1`] signer implementation.

use super::{LocalSigner, LocalSignerError};
use alloy_primitives::{Address, B256, B512, Signature, U256, hex};
use alloy_signer::{utils::raw_public_key_to_address, Result};
use secp256k1::{Message, PublicKey, SECP256K1, SecretKey, ecdsa::RecoveryId};
use rand::{CryptoRng, Rng};
use std::str::FromStr;

#[cfg(feature = "keystore")]
use std::path::Path;

impl LocalSigner<SecretKey> {
    /// Creates a new [`LocalSigner`] instance from a [`SecretKey`].
    pub fn from_secp256k1_secret_key(credential: SecretKey) -> Self {
        let address = secret_key_to_address(&credential);
        Self { credential, address, chain_id: None }
    }
    
    /// Creates a new [`LocalSigner`] instance from a raw scalar serialized as a [`B256`] byte
    /// array.
    ///
    /// This is identical to [`from_field_bytes`](Self::from_field_bytes).
    #[inline]
    pub fn from_bytes(bytes: &B256) -> Result<Self, secp256k1::Error> {
        SecretKey::from_slice(bytes.as_ref()).map(Self::from_secp256k1_secret_key)
    }
    
    /// Creates a new [`LocalSigner`] instance from a raw scalar serialized as a byte slice.
    ///
    /// Byte slices shorter than the field size (32 bytes) are handled by zero padding the input.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, secp256k1::Error> {
        SecretKey::from_slice(bytes).map(Self::from_secp256k1_secret_key)
    }
    
    /// Creates a new random keypair seeded with [`rand::thread_rng()`].
    #[inline]
    pub fn random() -> Self {
        Self::random_with(&mut rand::thread_rng())
    }
    
    /// Creates a new random keypair with provided RNG
    pub fn random_with<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        let (secret_key, _) = SECP256K1.generate_keypair(rng);
        Self::from_secp256k1_secret_key(secret_key)
    }

    /// Serialize this [`LocalSigner`]'s [`SigningKey`] as a [`B256`] byte array.
    #[inline]
    pub fn to_bytes(&self) -> B256 {
        B256::new(<[u8; 32]>::from(self.to_field_bytes()))
    }

    /// Serialize this [`LocalSigner`]'s [`SigningKey`] as a [`FieldBytes`] byte array.
    #[inline]
    pub fn to_field_bytes(&self) -> [u8; 32] {
        self.credential.secret_bytes()
    }

    /// Convenience function that returns this signer's ethereum public key as a [`B512`] byte
    /// array.
    #[inline]
    pub fn public_key(&self) -> B512 {
        B512::from_slice(&self.credential.public_key(SECP256K1).serialize_uncompressed()[1..])
    }
}


#[cfg(feature = "keystore")]
impl LocalSigner<SecretKey> {
    /// Creates a new random encrypted JSON with the provided password and stores it in the
    /// provided directory. Returns a tuple (LocalSigner, String) of the signer instance for the
    /// keystore with its random UUID. Accepts an optional name for the keystore file. If `None`,
    /// the keystore is stored as the stringified UUID.
    #[inline]
    pub fn new_keystore<P, R, S>(
        dir: P,
        rng: &mut R,
        password: S,
        name: Option<&str>,
    ) -> Result<(Self, String), LocalSignerError>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng,
        S: AsRef<[u8]>,
    {
        let (secret, uuid) = eth_keystore::new(dir, rng, password, name)?;
        Ok((Self::from_slice(&secret)?, uuid))
    }

    /// Decrypts an encrypted JSON from the provided path to construct a [`LocalSigner`] instance
    #[inline]
    pub fn decrypt_keystore<P, S>(keypath: P, password: S) -> Result<Self, LocalSignerError>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let secret = eth_keystore::decrypt_key(keypath, password)?;
        Ok(Self::from_slice(&secret)?)
    }

    /// Creates a new encrypted JSON with the provided private key and password and stores it in the
    /// provided directory. Returns a tuple (LocalSigner, String) of the signer instance for the
    /// keystore with its random UUID. Accepts an optional name for the keystore file. If `None`,
    /// the keystore is stored as the stringified UUID.
    #[inline]
    pub fn encrypt_keystore<P, R, B, S>(
        keypath: P,
        rng: &mut R,
        pk: B,
        password: S,
        name: Option<&str>,
    ) -> Result<(Self, String), LocalSignerError>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng,
        B: AsRef<[u8]>,
        S: AsRef<[u8]>,
    {
        let pk = pk.as_ref();
        let uuid = eth_keystore::encrypt_key(keypath, rng, pk, password, name)?;
        Ok((Self::from_slice(pk)?, uuid))
    }
}

impl PartialEq for LocalSigner<SecretKey> {
    fn eq(&self, other: &Self) -> bool {
        self.credential.secret_bytes().eq(&other.credential.secret_bytes())
            && self.address == other.address
            && self.chain_id == other.chain_id
    }
}

impl From<SecretKey> for LocalSigner<SecretKey> {
    fn from(value: SecretKey) -> Self {
        Self::from_secp256k1_secret_key(value)
    }
}

impl From<&SecretKey> for LocalSigner<SecretKey> {
    fn from(value: &SecretKey) -> Self {
        Self::from_secp256k1_secret_key(value.clone())
    }
}

impl FromStr for LocalSigner<SecretKey> {
    type Err = LocalSignerError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let array = hex::decode_to_array::<_, 32>(src)?;
        Ok(Self::from_slice(&array)?)
    }
}

fn secret_key_to_address(secret_key: &SecretKey) -> Address {
    let public_key = PublicKey::from_secret_key(SECP256K1, secret_key);
    let raw_public_key = &public_key.serialize_uncompressed()[1..];
    raw_public_key_to_address(&raw_public_key)
}


pub(crate) fn sign_hash_sync(secret_key: &SecretKey, hash: &B256) -> Result<Signature> {
    let msg = Message::from_digest(hash.0);
    let sig = SECP256K1.sign_ecdsa_recoverable(&msg, secret_key);
    let (rec_id, data) = sig.serialize_compact();
    
    Ok(Signature::new(
        U256::try_from_be_slice(&data[..32]).unwrap(),
        U256::try_from_be_slice(&data[32..64]).unwrap(),
        rec_id == RecoveryId::Zero,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    #[cfg(feature = "keystore")]
    use tempfile::tempdir;
    
    #[test]
    fn parse_pk() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";
        let _pk: LocalSigner<SecretKey> = s.parse().unwrap();
    }
}