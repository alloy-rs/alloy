//! [`secp256k1`] signer implementation.

use super::{LocalSigner, LocalSignerError};
use alloy_primitives::{Address, B256, B512, Signature, U256, hex};
use alloy_signer::{utils::raw_public_key_to_address, Result};
use secp256k1::{Message, PublicKey, SECP256K1, SecretKey};
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
        i32::from(rec_id) != 0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrivateKeySigner, SignerSync};
    use alloy_primitives::address;
    use alloy_primitives::b256;
    
    
    #[cfg(feature = "keystore")]
    use tempfile::tempdir;
    
    #[test]
    fn parse_pk() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";
        let _pk: PrivateKeySigner = s.parse().unwrap();
    }
    
    #[test]
    fn parse_short_key() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea3";
        assert!(s.len() < 64);
        let pk = s.parse::<PrivateKeySigner>().unwrap_err();
        match pk {
            LocalSignerError::HexError(hex::FromHexError::InvalidStringLength) => {}
            _ => panic!("Unexpected error"),
        }
    }
    
    #[cfg(feature = "keystore")]
    fn test_encrypted_json_keystore(key: LocalSigner<SecretKey>, uuid: &str, dir: &Path) {
        // sign a message using the given key
        let message = "Some data";
        let signature = key.sign_message_sync(message.as_bytes()).unwrap();

        // read from the encrypted JSON keystore and decrypt it, while validating that the
        // signatures produced by both the keys should match
        let path = Path::new(dir).join(uuid);
        let key2 = LocalSigner::<SecretKey>::decrypt_keystore(path.clone(), "randpsswd").unwrap();

        let signature2 = key2.sign_message_sync(message.as_bytes()).unwrap();
        assert_eq!(signature, signature2);

        std::fs::remove_file(&path).unwrap();
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

        let (key, uuid) = LocalSigner::<SecretKey>::encrypt_keystore(
            &dir,
            &mut rng,
            private_key,
            "randpsswd",
            None,
        )
        .unwrap();

        test_encrypted_json_keystore(key, &uuid, dir.path());
    }
    
    #[test]
    #[cfg(feature = "keystore-geth-compat")]
    fn test_encrypted_json_keystore_with_address() {
        // create and store an encrypted JSON keystore in this directory

        use std::fs::File;

        use eth_keystore::EthKeystore;
        let dir = tempdir().unwrap();
        let mut rng = rand::thread_rng();
        let (key, uuid) =
            LocalSigner::<SecretKey>::new_keystore(&dir, &mut rng, "randpsswd", None).unwrap();

        let path = Path::new(dir.path()).join(uuid.as_str());
        let file = File::open(path).unwrap();
        let keystore = serde_json::from_reader::<_, EthKeystore>(file).unwrap();

        assert!(!keystore.address.is_zero());

        test_encrypted_json_keystore(key, &uuid, dir.path());
    }
    
    #[test]
    fn signs_msg() {
        let message = "Some data";
        let hash = alloy_primitives::utils::eip191_hash_message(message);
        let key = LocalSigner::<SecretKey>::random_with(&mut rand::thread_rng());
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
        let signer = LocalSigner::random();
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
        let signer: LocalSigner<SecretKey> =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(signer.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));

        let signer: LocalSigner<SecretKey> =
            "0000000000000000000000000000000000000000000000000000000000000002".parse().unwrap();
        assert_eq!(signer.address, address!("2B5AD5c4795c026514f8317c7a215E218DcCD6cF"));

        let signer: LocalSigner<SecretKey> =
            "0000000000000000000000000000000000000000000000000000000000000003".parse().unwrap();
        assert_eq!(signer.address, address!("0x6813Eb9362372EEF6200f3b1dbC3f819671cBA69"));
    }
    
    #[test]
    fn conversions() {
        let key = b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let signer_b256: LocalSigner<SecretKey> = LocalSigner::from_bytes(&key).unwrap();
        assert_eq!(signer_b256.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));
        assert_eq!(signer_b256.chain_id, None);
        assert_eq!(signer_b256.credential, SecretKey::from_byte_array(&key.0).unwrap());

        let signer_str = LocalSigner::from_str(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap();
        assert_eq!(signer_str.address, signer_b256.address);
        assert_eq!(signer_str.chain_id, signer_b256.chain_id);
        assert_eq!(signer_str.credential, signer_b256.credential);
        assert_eq!(signer_str.to_bytes(), key);
        assert_eq!(signer_str.to_field_bytes().to_vec(), key.0.to_vec());

        let signer_slice = LocalSigner::from_slice(&key[..]).unwrap();
        assert_eq!(signer_slice.address, signer_b256.address);
        assert_eq!(signer_slice.chain_id, signer_b256.chain_id);
        assert_eq!(signer_slice.credential, signer_b256.credential);
        assert_eq!(signer_slice.to_bytes(), key);
        assert_eq!(signer_slice.to_field_bytes().to_vec(), key.0.to_vec());

        let signer_field_bytes = LocalSigner::from_bytes((&key.0).into()).unwrap();
        assert_eq!(signer_field_bytes.address, signer_b256.address);
        assert_eq!(signer_field_bytes.chain_id, signer_b256.chain_id);
        assert_eq!(signer_field_bytes.credential, signer_b256.credential);
        assert_eq!(signer_field_bytes.to_bytes(), key);
        assert_eq!(signer_field_bytes.to_field_bytes().to_vec(), key.0.to_vec());
    }
    
    #[test]
    fn key_from_str() {
        let signer: LocalSigner<SecretKey> =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();

        // Check FromStr and `0x`
        let signer_0x: LocalSigner<SecretKey> =
            "0x0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(signer.address, signer_0x.address);
        assert_eq!(signer.chain_id, signer_0x.chain_id);
        assert_eq!(signer.credential, signer_0x.credential);

        // Must fail because of `0z`
        "0z0000000000000000000000000000000000000000000000000000000000000001"
            .parse::<LocalSigner<SecretKey>>()
            .unwrap_err();
    }
    
    #[test]
    fn public_key() {
        let signer: LocalSigner<SecretKey> =
            "0x51fde55a7d696da3b318b21e231dec5ff4b33e895f191b2988e122e969b20e90".parse().unwrap();
        assert_eq!(signer.public_key(), B512::from_str("0x2bcb56445551cd344c9be67cfe27652932d7088c17b6c3c8dad622a5c8e8caf4574d68fa12355e7fefbe2377911016124b9284283527dd2ead05c7b6e5585fbd").unwrap());
    }
}
