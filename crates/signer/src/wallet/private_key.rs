//! [`k256`] wallet implementation.

use super::{Wallet, WalletError};
use crate::utils::secret_key_to_address;
use alloy_primitives::{hex, B256};
use k256::{
    ecdsa::{self, SigningKey},
    FieldBytes, NonZeroScalar, SecretKey as K256SecretKey,
};
use rand::{CryptoRng, Rng};
use std::str::FromStr;

#[cfg(feature = "keystore")]
use {elliptic_curve::rand_core, std::path::Path};

impl Wallet<SigningKey> {
    /// Creates a new Wallet instance from a raw scalar serialized as a [`B256`] byte array.
    ///
    /// This is identical to [`from_field_bytes`](Self::from_field_bytes).
    #[inline]
    pub fn from_bytes(bytes: &B256) -> Result<Self, ecdsa::Error> {
        Self::from_field_bytes((&bytes.0).into())
    }

    /// Creates a new Wallet instance from a raw scalar serialized as a [`FieldBytes`] byte array.
    #[inline]
    pub fn from_field_bytes(bytes: &FieldBytes) -> Result<Self, ecdsa::Error> {
        SigningKey::from_bytes(bytes).map(Self::new_pk)
    }

    /// Creates a new Wallet instance from a raw scalar serialized as a byte slice.
    ///
    /// Byte slices shorter than the field size (32 bytes) are handled by zero padding the input.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ecdsa::Error> {
        SigningKey::from_slice(bytes).map(Self::new_pk)
    }

    /// Creates a new random keypair seeded with [`rand::thread_rng()`].
    #[inline]
    pub fn random() -> Self {
        Self::random_with(&mut rand::thread_rng())
    }

    /// Creates a new random keypair seeded with the provided RNG.
    #[inline]
    pub fn random_with<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        Self::new_pk(SigningKey::random(rng))
    }

    #[inline]
    fn new_pk(signer: SigningKey) -> Self {
        let address = secret_key_to_address(&signer);
        Self::new_with_signer(signer, address, None)
    }

    /// Borrow the secret [`NonZeroScalar`] value for this key.
    ///
    /// # ⚠️ Warning
    ///
    /// This value is key material.
    ///
    /// Please treat it with the care it deserves!
    #[inline]
    pub fn as_nonzero_scalar(&self) -> &NonZeroScalar {
        self.signer.as_nonzero_scalar()
    }

    /// Serialize this [`Wallet`]'s [`SigningKey`] as a [`B256`] byte array.
    #[inline]
    pub fn to_bytes(&self) -> B256 {
        B256::new(<[u8; 32]>::from(self.to_field_bytes()))
    }

    /// Serialize this [`Wallet`]'s [`SigningKey`] as a [`FieldBytes`] byte array.
    #[inline]
    pub fn to_field_bytes(&self) -> FieldBytes {
        self.signer.to_bytes()
    }
}

#[cfg(feature = "keystore")]
impl Wallet<SigningKey> {
    /// Creates a new random encrypted JSON with the provided password and stores it in the
    /// provided directory. Returns a tuple (Wallet, String) of the wallet instance for the
    /// keystore with its random UUID. Accepts an optional name for the keystore file. If `None`,
    /// the keystore is stored as the stringified UUID.
    #[inline]
    pub fn new_keystore<P, R, S>(
        dir: P,
        rng: &mut R,
        password: S,
        name: Option<&str>,
    ) -> Result<(Self, String), WalletError>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng + rand_core::CryptoRng,
        S: AsRef<[u8]>,
    {
        let (secret, uuid) = eth_keystore::new(dir, rng, password, name)?;
        Ok((Self::from_slice(&secret)?, uuid))
    }

    /// Decrypts an encrypted JSON from the provided path to construct a Wallet instance
    #[inline]
    pub fn decrypt_keystore<P, S>(keypath: P, password: S) -> Result<Self, WalletError>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let secret = eth_keystore::decrypt_key(keypath, password)?;
        Ok(Self::from_slice(&secret)?)
    }

    /// Creates a new encrypted JSON with the provided private key and password and stores it in the
    /// provided directory. Returns a tuple (Wallet, String) of the wallet instance for the
    /// keystore with its random UUID. Accepts an optional name for the keystore file. If `None`,
    /// the keystore is stored as the stringified UUID.
    #[inline]
    pub fn encrypt_keystore<P, R, B, S>(
        keypath: P,
        rng: &mut R,
        pk: B,
        password: S,
        name: Option<&str>,
    ) -> Result<(Self, String), WalletError>
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

impl PartialEq for Wallet<SigningKey> {
    fn eq(&self, other: &Self) -> bool {
        self.signer.to_bytes().eq(&other.signer.to_bytes())
            && self.address == other.address
            && self.chain_id == other.chain_id
    }
}

impl From<SigningKey> for Wallet<SigningKey> {
    fn from(value: SigningKey) -> Self {
        Self::new_pk(value)
    }
}

impl From<K256SecretKey> for Wallet<SigningKey> {
    fn from(value: K256SecretKey) -> Self {
        Self::new_pk(value.into())
    }
}

impl FromStr for Wallet<SigningKey> {
    type Err = WalletError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let array = hex::decode_to_array::<_, 32>(src)?;
        Ok(Self::from_slice(&array)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocalWallet, Result, SignableTx, Signer, SignerSync};
    use alloy_consensus::TxLegacy;
    use alloy_primitives::{address, b256, ChainId, Signature, U256};

    #[cfg(feature = "keystore")]
    use tempfile::tempdir;

    #[test]
    fn parse_pk() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea30b";
        let _pk: Wallet<SigningKey> = s.parse().unwrap();
    }

    #[test]
    fn parse_short_key() {
        let s = "6f142508b4eea641e33cb2a0161221105086a84584c74245ca463a49effea3";
        assert!(s.len() < 64);
        let pk = s.parse::<LocalWallet>().unwrap_err();
        match pk {
            WalletError::HexError(hex::FromHexError::InvalidStringLength) => {}
            _ => panic!("Unexpected error"),
        }
    }

    #[cfg(feature = "keystore")]
    fn test_encrypted_json_keystore(key: Wallet<SigningKey>, uuid: &str, dir: &Path) {
        // sign a message using the given key
        let message = "Some data";
        let signature = key.sign_message_sync(message.as_bytes()).unwrap();

        // read from the encrypted JSON keystore and decrypt it, while validating that the
        // signatures produced by both the keys should match
        let path = Path::new(dir).join(uuid);
        let key2 = Wallet::<SigningKey>::decrypt_keystore(path.clone(), "randpsswd").unwrap();

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
        let (key, uuid) =
            Wallet::<SigningKey>::new_keystore(&dir, &mut rng, "randpsswd", None).unwrap();

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
            Wallet::<SigningKey>::encrypt_keystore(&dir, &mut rng, private_key, "randpsswd", None)
                .unwrap();

        test_encrypted_json_keystore(key, &uuid, dir.path());
    }

    #[test]
    fn signs_msg() {
        let message = "Some data";
        let hash = alloy_primitives::utils::eip191_hash_message(message);
        let key = Wallet::<SigningKey>::random_with(&mut rand::thread_rng());
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

    #[tokio::test]
    async fn signs_tx() {
        async fn sign_tx_test(tx: &mut TxLegacy, chain_id: Option<ChainId>) -> Result<Signature> {
            let mut before = tx.clone();
            let sig = sign_dyn_tx_test(tx, chain_id).await?;
            if let Some(chain_id) = chain_id {
                assert_eq!(tx.chain_id, Some(chain_id), "chain ID was not set");
                before.chain_id = Some(chain_id);
            }
            assert_eq!(*tx, before);
            Ok(sig)
        }

        async fn sign_dyn_tx_test(
            tx: &mut SignableTx,
            chain_id: Option<ChainId>,
        ) -> Result<Signature> {
            let mut wallet: Wallet<SigningKey> =
                "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318".parse().unwrap();
            wallet.set_chain_id(chain_id);

            let sig = wallet.sign_transaction_sync(tx)?;
            let sighash = tx.signature_hash();
            assert_eq!(sig.recover_address_from_prehash(&sighash).unwrap(), wallet.address);

            let sig_async = wallet.sign_transaction(tx).await.unwrap();
            assert_eq!(sig_async, sig);

            Ok(sig)
        }

        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction
        let mut tx = TxLegacy {
            to: alloy_consensus::TxKind::Call(address!("F0109fC8DF283027b6285cc889F5aA624EaC1F55")),
            value: U256::from(1_000_000_000),
            gas_limit: 2_000_000,
            nonce: 0,
            gas_price: 21_000_000_000,
            input: Default::default(),
            chain_id: None,
        };
        let sig_none = sign_tx_test(&mut tx, None).await.unwrap();

        tx.chain_id = Some(1);
        let sig_1 = sign_tx_test(&mut tx, None).await.unwrap();
        let expected = "c9cf86333bcb065d140032ecaab5d9281bde80f21b9687b3e94161de42d51895727a108a0b8d101465414033c3f705a9c7b826e596766046ee1183dbc8aeaa6825".parse().unwrap();
        assert_eq!(sig_1, expected);
        assert_ne!(sig_1, sig_none);

        tx.chain_id = Some(2);
        let sig_2 = sign_tx_test(&mut tx, None).await.unwrap();
        assert_ne!(sig_2, sig_1);
        assert_ne!(sig_2, sig_none);

        // Sets chain ID.
        tx.chain_id = None;
        let sig_none_none = sign_tx_test(&mut tx, None).await.unwrap();
        assert_eq!(sig_none_none, sig_none);

        tx.chain_id = None;
        let sig_none_1 = sign_tx_test(&mut tx, Some(1)).await.unwrap();
        assert_eq!(sig_none_1, sig_1);

        tx.chain_id = None;
        let sig_none_2 = sign_tx_test(&mut tx, Some(2)).await.unwrap();
        assert_eq!(sig_none_2, sig_2);

        // Errors on mismatch.
        tx.chain_id = Some(2);
        let error = sign_tx_test(&mut tx, Some(1)).await.unwrap_err();
        let expected_error = crate::Error::TransactionChainIdMismatch { signer: 1, tx: 2 };
        assert_eq!(error.to_string(), expected_error.to_string());
    }

    #[test]
    #[cfg(feature = "eip712")]
    fn typed_data() {
        use crate::Signer;
        use alloy_primitives::{keccak256, Address, I256, U256};
        use alloy_sol_types::{eip712_domain, sol, SolStruct};
        use alloy_dyn_abi::eip712::TypedData;
        use serde::Serialize;

        sol! {
            #[derive(Debug)]
            struct FooBar {
                int256 foo;
                uint256 bar;
                bytes fizz;
                bytes32 buzz;
                string far;
                address out;
            }
        }

        sol! {
            #[derive(Debug, Serialize)]
            struct FooBarNoFizz {
                int256 foo;
                uint256 bar;
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
            fizz: b"fizz".to_vec(),
            buzz: keccak256("buzz"),
            far: String::from("space"),
            out: Address::ZERO,
        };
        let wallet = Wallet::random();
        let hash = foo_bar.eip712_signing_hash(&domain);
        let sig = wallet.sign_typed_data_sync(&foo_bar, &domain).unwrap();
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), wallet.address());
        assert_eq!(wallet.sign_hash_sync(hash).unwrap(), sig);

        let foo_bar_no_fizz = FooBarNoFizz {
            foo: I256::try_from(10u64).unwrap(),
            bar: U256::from(20u64),
            buzz: keccak256("buzz"),
            far: String::from("space"),
            out: Address::ZERO,
        };
        
        let foo_bar_dynamic = TypedData::from_struct(&foo_bar_no_fizz, Some(domain.clone()));
        let no_fizz_hash = foo_bar_dynamic.eip712_signing_hash().unwrap();

        let sig_dynamic = wallet.sign_dynamic_typed_data_sync(&foo_bar_dynamic).unwrap();
        
        assert_eq!(sig_dynamic.recover_address_from_prehash(&no_fizz_hash).unwrap(), wallet.address());
        assert_eq!(wallet.sign_hash_sync(no_fizz_hash).unwrap(), sig_dynamic);
    }

    #[test]
    fn key_to_address() {
        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(wallet.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));

        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000002".parse().unwrap();
        assert_eq!(wallet.address, address!("2B5AD5c4795c026514f8317c7a215E218DcCD6cF"));

        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000003".parse().unwrap();
        assert_eq!(wallet.address, address!("6813Eb9362372EEF6200f3b1dbC3f819671cBA69"));
    }

    #[test]
    fn conversions() {
        let key = b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let wallet_b256: Wallet<SigningKey> = LocalWallet::from_bytes(&key).unwrap();
        assert_eq!(wallet_b256.address, address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf"));
        assert_eq!(wallet_b256.chain_id, None);
        assert_eq!(wallet_b256.signer, SigningKey::from_bytes((&key.0).into()).unwrap());

        let wallet_str =
            Wallet::from_str("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        assert_eq!(wallet_str.address, wallet_b256.address);
        assert_eq!(wallet_str.chain_id, wallet_b256.chain_id);
        assert_eq!(wallet_str.signer, wallet_b256.signer);
        assert_eq!(wallet_str.to_bytes(), key);
        assert_eq!(wallet_str.to_field_bytes(), key.0.into());

        let wallet_slice = Wallet::from_slice(&key[..]).unwrap();
        assert_eq!(wallet_slice.address, wallet_b256.address);
        assert_eq!(wallet_slice.chain_id, wallet_b256.chain_id);
        assert_eq!(wallet_slice.signer, wallet_b256.signer);
        assert_eq!(wallet_slice.to_bytes(), key);
        assert_eq!(wallet_slice.to_field_bytes(), key.0.into());

        let wallet_field_bytes = Wallet::from_field_bytes((&key.0).into()).unwrap();
        assert_eq!(wallet_field_bytes.address, wallet_b256.address);
        assert_eq!(wallet_field_bytes.chain_id, wallet_b256.chain_id);
        assert_eq!(wallet_field_bytes.signer, wallet_b256.signer);
        assert_eq!(wallet_field_bytes.to_bytes(), key);
        assert_eq!(wallet_field_bytes.to_field_bytes(), key.0.into());
    }

    #[test]
    fn key_from_str() {
        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();

        // Check FromStr and `0x`
        let wallet_0x: Wallet<SigningKey> =
            "0x0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(wallet.address, wallet_0x.address);
        assert_eq!(wallet.chain_id, wallet_0x.chain_id);
        assert_eq!(wallet.signer, wallet_0x.signer);

        // Must fail because of `0z`
        "0z0000000000000000000000000000000000000000000000000000000000000001"
            .parse::<Wallet<SigningKey>>()
            .unwrap_err();
    }
}
