//! [`secp256k1`] signer implementation.
//!
//! This module provides a signer implementation using the [`secp256k1`] crate
//! as an alternative to the default [`k256`] implementation.

use crate::{LocalSigner, LocalSignerError};
use alloy_primitives::{hex, keccak256, Address, B256, B512};
use alloy_signer::Signer;
use k256::ecdsa::{
    signature::{hazmat::PrehashSigner, Error as SignatureError},
    RecoveryId, Signature as K256Signature,
};
use rand::{CryptoRng, Rng};
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey, SECP256K1};
use std::str::FromStr;

#[cfg(feature = "keystore")]
use std::path::Path;

/// A wrapper around [`secp256k1::SecretKey`] that implements [`PrehashSigner`].
///
/// This allows using the `secp256k1` crate for ECDSA operations while maintaining
/// compatibility with the [`LocalSigner`] infrastructure.
#[derive(Clone)]
pub struct Secp256k1Credential(SecretKey);

impl Secp256k1Credential {
    /// Creates a new [`Secp256k1Credential`] from a [`SecretKey`].
    #[inline]
    pub const fn new(secret_key: SecretKey) -> Self {
        Self(secret_key)
    }

    /// Returns a reference to the inner [`SecretKey`].
    #[inline]
    pub const fn inner(&self) -> &SecretKey {
        &self.0
    }

    /// Consumes this credential and returns the inner [`SecretKey`].
    #[inline]
    pub const fn into_inner(self) -> SecretKey {
        self.0
    }

    /// Returns the public key for this credential.
    #[inline]
    pub fn public_key(&self) -> PublicKey {
        self.0.public_key(SECP256K1)
    }
}

impl std::fmt::Debug for Secp256k1Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secp256k1Credential").finish_non_exhaustive()
    }
}

impl PartialEq for Secp256k1Credential {
    fn eq(&self, other: &Self) -> bool {
        self.0.secret_bytes() == other.0.secret_bytes()
    }
}

impl From<SecretKey> for Secp256k1Credential {
    fn from(secret_key: SecretKey) -> Self {
        Self::new(secret_key)
    }
}

impl PrehashSigner<(K256Signature, RecoveryId)> for Secp256k1Credential {
    fn sign_prehash(&self, prehash: &[u8]) -> Result<(K256Signature, RecoveryId), SignatureError> {
        let msg =
            Message::from_digest_slice(prehash).map_err(|_| SignatureError::from_source(""))?;

        let sig = SECP256K1.sign_ecdsa_recoverable(&msg, &self.0);
        let (rec_id, data) = sig.serialize_compact();

        // Convert secp256k1 signature to k256 signature
        let k256_sig = K256Signature::from_slice(&data).map_err(SignatureError::from_source)?;

        let k256_rec_id = RecoveryId::from_byte(i32::from(rec_id) as u8)
            .ok_or_else(|| SignatureError::from_source("invalid recovery id"))?;

        Ok((k256_sig, k256_rec_id))
    }
}

/// Converts a [`secp256k1::SecretKey`] to its corresponding Ethereum address.
#[inline]
fn secret_key_to_address(secret_key: &SecretKey) -> Address {
    let public = secret_key.public_key(SECP256K1);
    public_key_to_address(&public)
}

/// Converts a [`secp256k1::PublicKey`] to its corresponding Ethereum address.
#[inline]
fn public_key_to_address(public: &PublicKey) -> Address {
    // Strip out the first byte (0x04 tag for uncompressed public key)
    let hash = keccak256(&public.serialize_uncompressed()[1..]);
    Address::from_slice(&hash[12..])
}

impl LocalSigner<Secp256k1Credential> {
    /// Creates a new [`LocalSigner`] instance from a [`secp256k1::SecretKey`].
    #[doc(alias = "from_private_key")]
    #[doc(alias = "new_private_key")]
    #[doc(alias = "new_pk")]
    #[inline]
    pub fn from_secp256k1(secret_key: SecretKey) -> Self {
        let address = secret_key_to_address(&secret_key);
        Self::new_with_credential(Secp256k1Credential::new(secret_key), address, None)
    }

    /// Creates a new [`LocalSigner`] instance from a raw scalar serialized as a [`B256`] byte
    /// array.
    #[inline]
    pub fn from_bytes(bytes: &B256) -> Result<Self, secp256k1::Error> {
        Self::from_slice(bytes.as_slice())
    }

    /// Creates a new [`LocalSigner`] instance from a raw scalar serialized as a byte slice.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, secp256k1::Error> {
        SecretKey::from_slice(bytes).map(Self::from_secp256k1)
    }

    /// Creates a new random keypair seeded with [`rand::thread_rng()`].
    #[inline]
    pub fn random() -> Self {
        Self::random_with(&mut rand::thread_rng())
    }

    /// Creates a new random keypair seeded with the provided RNG.
    #[inline]
    pub fn random_with<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(rng);
        Self::from_secp256k1(secret_key)
    }

    /// Serialize this [`LocalSigner`]'s [`SecretKey`] as a [`B256`] byte array.
    #[inline]
    pub fn to_bytes(&self) -> B256 {
        B256::from_slice(&self.credential.0.secret_bytes())
    }

    /// Convenience function that returns this signer's ethereum public key as a [`B512`] byte
    /// array.
    #[inline]
    pub fn public_key(&self) -> B512 {
        let public = self.credential.public_key();
        // Remove the 0x04 prefix byte
        B512::from_slice(&public.serialize_uncompressed()[1..])
    }

    /// Converts this `Secp256k1Signer` to a [`PrivateKeySigner`](crate::PrivateKeySigner)
    /// (k256-based).
    ///
    /// The resulting signer will have the same address, private key, and chain ID.
    #[inline]
    pub fn to_k256(&self) -> crate::PrivateKeySigner {
        let mut signer = crate::PrivateKeySigner::from_slice(&self.credential.0.secret_bytes())
            .expect("valid secp256k1 key bytes should be valid k256 key bytes");
        signer.set_chain_id(self.chain_id);
        signer
    }

    /// Converts this `Secp256k1Signer` into a [`PrivateKeySigner`](crate::PrivateKeySigner)
    /// (k256-based).
    ///
    /// This is the consuming version of [`to_k256`](Self::to_k256).
    #[inline]
    pub fn into_k256(self) -> crate::PrivateKeySigner {
        self.to_k256()
    }
}

#[cfg(feature = "keystore")]
impl LocalSigner<Secp256k1Credential> {
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

impl PartialEq for LocalSigner<Secp256k1Credential> {
    fn eq(&self, other: &Self) -> bool {
        self.credential == other.credential
            && self.address == other.address
            && self.chain_id == other.chain_id
    }
}

impl From<SecretKey> for LocalSigner<Secp256k1Credential> {
    fn from(value: SecretKey) -> Self {
        Self::from_secp256k1(value)
    }
}

impl FromStr for LocalSigner<Secp256k1Credential> {
    type Err = LocalSignerError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let array = hex::decode_to_array::<_, 32>(src)?;
        Ok(Self::from_slice(&array)?)
    }
}

impl From<crate::PrivateKeySigner> for LocalSigner<Secp256k1Credential> {
    fn from(signer: crate::PrivateKeySigner) -> Self {
        signer.into_secp256k1()
    }
}

impl From<&crate::PrivateKeySigner> for LocalSigner<Secp256k1Credential> {
    fn from(signer: &crate::PrivateKeySigner) -> Self {
        signer.to_secp256k1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrivateKeySigner, Secp256k1Signer, SignerSync};
    use alloy_primitives::{address, b256};

    #[cfg(feature = "keystore")]
    use tempfile::tempdir;

    #[test]
    fn parse_pk() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";
        let _pk: Secp256k1Signer = s.parse().unwrap();
    }

    #[test]
    fn parse_short_key() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea3";
        assert!(s.len() < 64);
        let pk = s.parse::<Secp256k1Signer>().unwrap_err();
        match pk {
            LocalSignerError::HexError(hex::FromHexError::InvalidStringLength) => {}
            _ => panic!("Unexpected error"),
        }
    }

    #[cfg(feature = "keystore")]
    fn test_encrypted_json_keystore(key: Secp256k1Signer, uuid: &str, dir: &Path) {
        use std::path::Path;

        // sign a message using the given key
        let message = "Some data";
        let signature = key.sign_message_sync(message.as_bytes()).unwrap();

        // read from the encrypted JSON keystore and decrypt it, while validating that the
        // signatures produced by both the keys should match
        let path = Path::new(dir).join(uuid);
        let key2 = Secp256k1Signer::decrypt_keystore(path.clone(), "randpsswd").unwrap();

        let signature2 = key2.sign_message_sync(message.as_bytes()).unwrap();
        assert_eq!(signature, signature2);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    #[cfg(feature = "keystore")]
    fn encrypted_json_keystore_new() {
        // create and store an encrypted JSON keystore in this directory
        let dir = tempdir().unwrap();
        let mut rng = rand::thread_rng();
        let (key, uuid) = Secp256k1Signer::new_keystore(&dir, &mut rng, "randpsswd", None).unwrap();

        test_encrypted_json_keystore(key, &uuid, dir.path());
    }

    #[test]
    #[cfg(feature = "keystore")]
    fn encrypted_json_keystore_from_pk() {
        // create and store an encrypted JSON keystore in this directory
        let dir = tempdir().unwrap();
        let mut rng = rand::thread_rng();

        let private_key =
            hex::decode("6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b")
                .unwrap();

        let (key, uuid) =
            Secp256k1Signer::encrypt_keystore(&dir, &mut rng, private_key, "randpsswd", None)
                .unwrap();

        test_encrypted_json_keystore(key, &uuid, dir.path());
    }

    #[test]
    fn signs_msg() {
        let message = "Some data";
        let hash = alloy_primitives::utils::eip191_hash_message(message);
        let key = Secp256k1Signer::random_with(&mut rand::thread_rng());
        let address = key.address;

        // sign a message
        let signature = key.sign_message_sync(message.as_bytes()).unwrap();

        // ecrecover via the message will hash internally
        let recovered = signature.recover_address_from_msg(message).unwrap();
        assert_eq!(recovered, address);

        // if provided with a hash, it will skip hashing
        let recovered2 = signature.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, address);
    }

    #[test]
    #[cfg(feature = "eip712")]
    fn typed_data() {
        use alloy_dyn_abi::eip712::TypedData;
        use alloy_primitives::{keccak256, Address, I256, U256};
        use alloy_sol_types::{eip712_domain, sol, SolStruct};
        use serde::Serialize;

        sol! {
            #[derive(Debug, Serialize)]
            struct FooBar {
                int256 foo;
                uint256 bar;
                bytes fizz;
                bytes32 buzz;
                string far;
                address out;
            }
        }

        let domain = eip712_domain! {
            name: "Eip712Test",
            version: "1",
            chain_id: 1,
            verifying_contract: address!("0000000000000000000000000000000000000001"),
            salt: keccak256("eip712-test-75F0CCte"),
        };
        let foo_bar = FooBar {
            foo: I256::try_from(10u64).unwrap(),
            bar: U256::from(20u64),
            fizz: b"fizz".to_vec().into(),
            buzz: keccak256("buzz"),
            far: "space".into(),
            out: Address::ZERO,
        };
        let signer = LocalSigner::<Secp256k1Credential>::random();
        let hash = foo_bar.eip712_signing_hash(&domain);
        let sig = signer.sign_typed_data_sync(&foo_bar, &domain).unwrap();
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), signer.address());
        assert_eq!(signer.sign_hash_sync(&hash).unwrap(), sig);
        let foo_bar_dynamic = TypedData::from_struct(&foo_bar, Some(domain));
        let dynamic_hash = foo_bar_dynamic.eip712_signing_hash().unwrap();
        let sig_dynamic = signer.sign_dynamic_typed_data_sync(&foo_bar_dynamic).unwrap();
        assert_eq!(
            sig_dynamic.recover_address_from_prehash(&dynamic_hash).unwrap(),
            signer.address()
        );
        assert_eq!(signer.sign_hash_sync(&dynamic_hash).unwrap(), sig_dynamic);
    }

    #[test]
    fn key_to_address() {
        let signer: Secp256k1Signer =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(signer.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));

        let signer: Secp256k1Signer =
            "0000000000000000000000000000000000000000000000000000000000000002".parse().unwrap();
        assert_eq!(signer.address, address!("2B5AD5c4795c026514f8317c7a215E218DcCD6cF"));

        let signer: Secp256k1Signer =
            "0000000000000000000000000000000000000000000000000000000000000003".parse().unwrap();
        assert_eq!(signer.address, address!("0x6813Eb9362372EEF6200f3b1dbC3f819671cBA69"));
    }

    #[test]
    fn conversions() {
        let key = b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let signer_b256: Secp256k1Signer = Secp256k1Signer::from_bytes(&key).unwrap();
        assert_eq!(signer_b256.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));
        assert_eq!(signer_b256.chain_id, None);

        let signer_str = Secp256k1Signer::from_str(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap();
        assert_eq!(signer_str.address, signer_b256.address);
        assert_eq!(signer_str.chain_id, signer_b256.chain_id);
        assert_eq!(signer_str.to_bytes(), key);

        let signer_slice = Secp256k1Signer::from_slice(&key[..]).unwrap();
        assert_eq!(signer_slice.address, signer_b256.address);
        assert_eq!(signer_slice.chain_id, signer_b256.chain_id);
        assert_eq!(signer_slice.to_bytes(), key);
    }

    #[test]
    fn key_from_str() {
        let signer: Secp256k1Signer =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();

        // Check FromStr and `0x`
        let signer_0x: Secp256k1Signer =
            "0x0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(signer.address, signer_0x.address);
        assert_eq!(signer.chain_id, signer_0x.chain_id);

        // Must fail because of `0z`
        "0z0000000000000000000000000000000000000000000000000000000000000001"
            .parse::<Secp256k1Signer>()
            .unwrap_err();
    }

    #[test]
    fn public_key() {
        let signer: Secp256k1Signer =
            "0x51fde55a7d696da3b318b21e231dec5ff4b33e895f191b2988e122e969b20e90".parse().unwrap();
        assert_eq!(signer.public_key(), B512::from_str("0x2bcb56445551cd344c9be67cfe27652932d7088c17b6c3c8dad622a5c8e8caf4574d68fa12355e7fefbe2377911016124b9284283527dd2ead05c7b6e5585fbd").unwrap());
    }

    /// Test that the secp256k1 and k256 implementations produce the same results.
    #[test]
    fn k256_secp256k1_compatibility() {
        let key_hex = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";

        let k256_signer: PrivateKeySigner = key_hex.parse().unwrap();
        let secp256k1_signer: Secp256k1Signer = key_hex.parse().unwrap();

        // Same address
        assert_eq!(k256_signer.address(), secp256k1_signer.address());

        // Same public key
        assert_eq!(k256_signer.public_key(), secp256k1_signer.public_key());

        // Signatures can be recovered to the same address
        let message = b"test message";
        let k256_sig = k256_signer.sign_message_sync(message).unwrap();
        let secp256k1_sig = secp256k1_signer.sign_message_sync(message).unwrap();

        assert_eq!(
            k256_sig.recover_address_from_msg(message).unwrap(),
            secp256k1_sig.recover_address_from_msg(message).unwrap()
        );
    }

    #[test]
    fn test_parity() {
        let signer = Secp256k1Signer::random();
        let message = b"hello";
        let signature = signer.sign_message_sync(message).unwrap();
        let value = signature.as_bytes().to_vec();
        let recovered_signature: alloy_primitives::Signature = value.as_slice().try_into().unwrap();
        assert_eq!(signature, recovered_signature);
    }

    #[test]
    fn test_k256_to_secp256k1_conversion() {
        let k256_signer: PrivateKeySigner =
            "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b".parse().unwrap();

        // Test to_secp256k1
        let secp_signer = k256_signer.to_secp256k1();
        assert_eq!(k256_signer.address(), secp_signer.address());
        assert_eq!(k256_signer.public_key(), secp_signer.public_key());

        // Test into_secp256k1
        let secp_signer2: Secp256k1Signer = k256_signer.clone().into_secp256k1();
        assert_eq!(secp_signer.address(), secp_signer2.address());

        // Test From trait
        let secp_signer3: Secp256k1Signer = k256_signer.clone().into();
        assert_eq!(secp_signer.address(), secp_signer3.address());

        // Test From<&PrivateKeySigner>
        let secp_signer4: Secp256k1Signer = (&k256_signer).into();
        assert_eq!(secp_signer.address(), secp_signer4.address());
    }

    #[test]
    fn test_secp256k1_to_k256_conversion() {
        let secp_signer: Secp256k1Signer =
            "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b".parse().unwrap();

        // Test to_k256
        let k256_signer = secp_signer.to_k256();
        assert_eq!(secp_signer.address(), k256_signer.address());
        assert_eq!(secp_signer.public_key(), k256_signer.public_key());

        // Test into_k256
        let k256_signer2: PrivateKeySigner = secp_signer.clone().into_k256();
        assert_eq!(k256_signer.address(), k256_signer2.address());

        // Test From trait
        let k256_signer3: PrivateKeySigner = secp_signer.clone().into();
        assert_eq!(k256_signer.address(), k256_signer3.address());

        // Test From<&Secp256k1Signer>
        let k256_signer4: PrivateKeySigner = (&secp_signer).into();
        assert_eq!(k256_signer.address(), k256_signer4.address());
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original: PrivateKeySigner =
            "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b".parse().unwrap();

        // k256 -> secp256k1 -> k256
        let secp = original.to_secp256k1();
        let roundtrip = secp.to_k256();

        assert_eq!(original.address(), roundtrip.address());
        assert_eq!(original.to_bytes(), roundtrip.to_bytes());

        // Verify signatures match
        let message = b"roundtrip test";
        let sig1 = original.sign_message_sync(message).unwrap();
        let sig2 = roundtrip.sign_message_sync(message).unwrap();
        assert_eq!(
            sig1.recover_address_from_msg(message).unwrap(),
            sig2.recover_address_from_msg(message).unwrap()
        );
    }

    #[test]
    fn test_chain_id_preserved_in_conversion() {
        let mut k256_signer: PrivateKeySigner =
            "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b".parse().unwrap();
        k256_signer.set_chain_id(Some(1337));

        let secp_signer = k256_signer.to_secp256k1();
        assert_eq!(secp_signer.chain_id(), Some(1337));

        let back_to_k256 = secp_signer.to_k256();
        assert_eq!(back_to_k256.chain_id(), Some(1337));
    }
}
