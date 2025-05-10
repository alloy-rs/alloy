use std::fmt;
use std::path::PathBuf;

use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, utils::eip191_message, Address, ChainId, Signature, B256};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
use iota_stronghold::{procedures::KeyType, KeyProvider, Location, SnapshotPath, Stronghold};

const STRONGHOLD_PATH: &str = "signer.stronghold";
const CLIENT_PATH: &[u8] = b"client-path-0";
const VAULT_PATH: &[u8] = b"vault-path";
const RECORD_PATH: &[u8] = b"record-path-0";

/// StrongholdSigner uses the Stronghold vault as the secure backing for an Ethereum Signer.
///
#[derive(Clone)]
pub struct StrongholdSigner {
    address: Address,
    chain_id: Option<ChainId>,
    stronghold: iota_stronghold::Stronghold,
}

impl fmt::Debug for StrongholdSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StrongholdSigner")
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StrongholdSignerError {
    /// [`hex`] error.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// [`iota_stronghold::types::ClientError`] error.
    #[error(transparent)]
    Client(#[from] iota_stronghold::types::ClientError),
    /// [`alloy_primitives::SignatureError`] error.
    #[error(transparent)]
    Signature(#[from] alloy_primitives::SignatureError),
    /// [`iota_stronghold::procedures::ProcedureError`] error.
    #[error(transparent)]
    Procedure(#[from] iota_stronghold::procedures::ProcedureError),
    /// [`std::env::VarError`] error.
    #[error(transparent)]
    Var(#[from] std::env::VarError),
    /// Invalid recovery value.
    #[error("invalid recovery value: {0}")]
    InvalidRecoveryValue(u8),
    /// Invalid signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    /// Invalid signature bytes.
    #[error("invalid signature bytes: {0}")]
    InvalidSignatureBytes(String),
    /// k256::ecdsa::Error
    #[error(transparent)]
    K256Error(#[from] k256::ecdsa::Error),
    /// Unsupported operation.
    #[error(transparent)]
    UnsupportedOperation(#[from] alloy_signer::Error),
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl alloy_network::TxSigner<Signature> for StrongholdSigner {
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(
            self,
            tx,
            self.sign_using_stronghold(tx.encoded_for_signing())
        )
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for StrongholdSigner {
    #[inline]
    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        return Err(alloy_signer::Error::UnsupportedOperation(
            alloy_signer::UnsupportedSignerOperation::SignHash,
        ));
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        let prefixed_msg = eip191_message(message);
        let sig = self
            .sign_using_stronghold(prefixed_msg)
            .map_err(alloy_signer::Error::other)?;
        Ok(sig)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

alloy_network::impl_into_wallet!(StrongholdSigner);

impl StrongholdSigner {
    /// Create a new StrongholdSigner with an optional chain ID.
    ///
    /// This will read the passphrase from the `PASSPHRASE` environment variable.
    /// This passphrase should be treated with the same level of security as a private key.
    ///
    /// If the stronghold snapshot file doesn't exist, it will create a new key.
    ///
    pub fn new(chain_id: Option<ChainId>) -> Result<Self, StrongholdSignerError> {
        let passphrase = std::env::var("PASSPHRASE")?.as_bytes().to_vec();
        Self::initialize(&STRONGHOLD_PATH.into(), passphrase, chain_id)
    }

    /// Create a new StrongholdSigner with a custom path and an optional chain ID.
    ///
    /// This will read the passphrase from the `PASSPHRASE` environment variable.
    /// This passphrase should be treated with the same level of security as a private key.
    ///
    /// If the stronghold snapshot file doesn't exist, it will create a new key.
    ///
    pub fn new_from_path(
        stronghold_path: PathBuf,
        chain_id: Option<ChainId>,
    ) -> Result<Self, StrongholdSignerError> {
        let passphrase = std::env::var("PASSPHRASE")?.as_bytes().to_vec();
        Self::initialize(&stronghold_path, passphrase, chain_id)
    }

    /// Helper method to initialize a StrongholdSigner with the given path, passphrase, and chain ID.
    fn initialize(
        stronghold_path: &PathBuf,
        passphrase: Vec<u8>,
        chain_id: Option<ChainId>,
    ) -> Result<Self, StrongholdSignerError> {
        let key_provider = KeyProvider::with_passphrase_hashed_blake2b(passphrase)?;
        let stronghold = Stronghold::default();
        let snapshot_path = SnapshotPath::from_path(stronghold_path);

        let init_result =
            stronghold.load_client_from_snapshot(CLIENT_PATH, &key_provider, &snapshot_path);

        let address = match init_result {
            Err(iota_stronghold::ClientError::SnapshotFileMissing(_)) => {
                // No snapshot file exists, create a new client and key
                stronghold.create_client(CLIENT_PATH)?;
                Self::maybe_generate_key(
                    &stronghold,
                    &key_provider,
                    KeyType::Secp256k1Ecdsa,
                    stronghold_path.to_path_buf(),
                )?;

                stronghold.commit_with_keyprovider(&snapshot_path, &key_provider)?;
                Self::get_evm_address(&stronghold)?
            }
            Err(iota_stronghold::ClientError::ClientAlreadyLoaded(_)) => {
                // Client already loaded, get the address
                stronghold.get_client(CLIENT_PATH)?;
                Self::get_evm_address(&stronghold)?
            }
            _ => Self::get_evm_address(&stronghold)?,
        };

        Ok(Self {
            address,
            chain_id,
            stronghold,
        })
    }

    /// Creates a new StrongholdSigner from an existing Stronghold instance with the key already in place.
    pub fn from_stronghold(
        stronghold: Stronghold,
        chain_id: Option<ChainId>,
    ) -> Result<Self, StrongholdSignerError> {
        stronghold.get_client(CLIENT_PATH)?;
        let address = Self::get_evm_address(&stronghold)?;
        Ok(Self {
            address,
            chain_id,
            stronghold,
        })
    }

    /// Creates a key if it doesn't already exist in the stronghold vault
    fn maybe_generate_key(
        stronghold: &Stronghold,
        key_provider: &KeyProvider,
        ty: KeyType,
        stronghold_path: PathBuf,
    ) -> Result<(), StrongholdSignerError> {
        let output = Location::const_generic(VAULT_PATH.to_vec(), RECORD_PATH.to_vec());

        let client = stronghold.get_client(CLIENT_PATH)?;
        match client.record_exists(&output) {
            Ok(exists) if exists => {
                // Key already exists, do nothing
            }
            Ok(exists) if !exists => {
                // No key exists, generate one
                let generate_key_procedure =
                    iota_stronghold::procedures::GenerateKey { ty, output };
                client.execute_procedure(generate_key_procedure)?;
                let snapshot_path = SnapshotPath::from_path(stronghold_path);
                stronghold.commit_with_keyprovider(&snapshot_path, key_provider)?;
            }
            Ok(_) => unreachable!(),
            Err(_) => {
                // Handle error by attempting to generate the key
                let generate_key_procedure =
                    iota_stronghold::procedures::GenerateKey { ty, output };
                client.execute_procedure(generate_key_procedure)?;
                let snapshot_path = SnapshotPath::from_path(stronghold_path);
                stronghold.commit_with_keyprovider(&snapshot_path, key_provider)?;
            }
        }

        Ok(())
    }

    /// Gets the Ethereum address associated with the key in stronghold
    fn get_evm_address(stronghold: &Stronghold) -> Result<Address, StrongholdSignerError> {
        let client = stronghold.get_client(CLIENT_PATH)?;
        let private_key = Location::const_generic(VAULT_PATH.to_vec(), RECORD_PATH.to_vec());
        let result =
            client.execute_procedure(iota_stronghold::procedures::GetEvmAddress { private_key })?;

        Ok(result.into())
    }

    /// Sign a message using the Stronghold client.
    /// The private key is never exposed outside of Stronghold's secure enclave.
    ///
    /// This returns an alloy_primitives::Signature with the correct format.
    ///
    fn sign_using_stronghold(&self, msg: Vec<u8>) -> Result<Signature, StrongholdSignerError> {
        let client = self.stronghold.get_client(CLIENT_PATH)?;
        let location = Location::const_generic(VAULT_PATH.to_vec(), RECORD_PATH.to_vec());

        // Sign the message using the Stronghold secp256k1 ECDSA procedure
        let result_bytes: [u8; 65] =
            client.execute_procedure(iota_stronghold::procedures::Secp256k1EcdsaSign {
                flavor: iota_stronghold::procedures::Secp256k1EcdsaFlavor::Keccak256,
                msg,
                private_key: location.clone(),
            })?;

        let sig = k256::ecdsa::Signature::from_slice(&result_bytes[..64])
            .map_err(StrongholdSignerError::K256Error)?;
        let rid = k256::ecdsa::RecoveryId::from_byte(result_bytes[64]).ok_or(
            StrongholdSignerError::InvalidSignatureBytes(hex::encode(result_bytes)),
        )?;

        let signature = Signature::from((sig, rid));
        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Address;
    use alloy_consensus::{TxEnvelope, TxLegacy};
    use alloy_network::TxSigner;
    use alloy_primitives::{bytes, U256};
    use alloy_signer::Signer;
    use std::env;
    use std::fs;

    // Helper to setup test environment and return a StrongholdSigner
    fn setup_test_env(chain_id: Option<ChainId>) -> StrongholdSigner {
        env::set_var("PASSPHRASE", "test_passphrase_of_sufficient_length");

        // Create a new signer directly
        StrongholdSigner::new(chain_id).expect("Failed to create StrongholdSigner")
    }

    // Helper to clean up test environment
    fn cleanup_test_env() {
        env::remove_var("PASSPHRASE");
    }

    // Helper to setup test environment with a specific path and return a StrongholdSigner
    fn setup_test_env_with_path(path: PathBuf, chain_id: Option<ChainId>) -> StrongholdSigner {
        env::set_var("PASSPHRASE", "test_passphrase_of_sufficient_length");

        // Remove the file if it exists
        if path.exists() {
            fs::remove_file(&path).expect("Failed to remove existing file");
        }

        // Create a new signer with the specified path
        StrongholdSigner::new_from_path(path, chain_id).expect("Failed to create StrongholdSigner")
    }

    #[tokio::test]
    async fn test_initialize_new_signer() {
        let signer = setup_test_env(Some(1));

        assert!(signer.address != Address::ZERO, "Address should be set");
        assert_eq!(signer.chain_id, Some(1), "Chain ID should match");

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_reinitialize_existing_signer() {
        let signer1 = setup_test_env(Some(1));
        let address1 = signer1.address;

        // Second creation should load same key
        let signer2 = setup_test_env(Some(1));
        assert_eq!(signer2.address, address1, "Should load same address");

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_sign_message() {
        let signer = setup_test_env(Some(1));

        let signer_address = alloy_network::TxSigner::address(&signer);
        let message = b"hello world";
        let signature = signer
            .sign_message(message)
            .await
            .expect("Failed to sign message");

        // Recover address from the signature
        let recovered = signature
            .recover_address_from_msg(message)
            .expect("Failed to recover address");
        assert_eq!(signer_address, recovered);

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_sign_hash() {
        let signer = setup_test_env(Some(1));

        let message = b"hello world";
        let hash = alloy::primitives::keccak256(message);
        let signature = signer.sign_hash(&hash).await;

        assert!(
            signature.is_err(),
            "Should return UnsupportedOperation error"
        );

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_sign_transaction() {
        let signer = setup_test_env(Some(1));

        let to = "deaddeaddeaddeaddeaddeaddeaddeaddeaddead";
        let to: Address = to.parse().unwrap();

        let mut tx = TxLegacy {
            to: alloy::primitives::TxKind::Call(to),
            value: U256::from(100),
            gas_price: 1,
            gas_limit: 21000,
            input: bytes!(""),
            nonce: 0,
            ..Default::default()
        };

        let result = signer.sign_transaction(&mut tx).await;
        assert!(result.is_ok(), "Should sign transaction successfully");

        let sig = result.unwrap();
        let _envelope = TxEnvelope::Legacy(tx.into_signed(sig));

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_get_evm_address() {
        let signer = setup_test_env(Some(1));

        let tx_signer_addr: Address = TxSigner::address(&signer);
        assert_ne!(tx_signer_addr, Address::ZERO, "Address should not be zero");
        assert_eq!(tx_signer_addr.len(), 20, "Address should be 20 bytes");

        let signer_address: Address = alloy_signer::Signer::address(&signer);
        assert_ne!(signer_address, Address::ZERO, "Address should not be zero");
        assert_eq!(signer_address.len(), 20, "Address should be 20 bytes");

        assert_eq!(tx_signer_addr, signer_address, "Addresses should match");

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_chain_id_management() {
        let mut signer = setup_test_env(Some(1));

        assert_eq!(signer.chain_id(), Some(1));

        signer.set_chain_id(Some(5));
        assert_eq!(signer.chain_id(), Some(5));

        signer.set_chain_id(None);
        assert_eq!(signer.chain_id(), None);

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_missing_passphrase() {
        env::remove_var("PASSPHRASE");

        let result = StrongholdSigner::new(Some(1));
        assert!(result.is_err(), "Should fail without passphrase");
    }

    #[tokio::test]
    async fn test_signer_trait_implementation() {
        let signer = setup_test_env(Some(1));

        // Test address method
        let address: Address = TxSigner::address(&signer);
        assert_ne!(address, Address::ZERO);

        // Test chain_id method
        assert_eq!(signer.chain_id(), Some(1));

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_end_to_end_transaction_with_anvil() {
        use alloy::network::EthereumWallet;
        use alloy::node_bindings::Anvil;
        use alloy::primitives::address;
        use alloy::providers::{ext::AnvilApi, Provider, ProviderBuilder};
        use alloy::rpc::types::TransactionRequest;
        use alloy_network::TransactionBuilder;
        use alloy_primitives::U256;

        let anvil = Anvil::new().spawn();
        let chain_id = anvil.chain_id();
        let signer = setup_test_env(Some(chain_id));

        let sender_address: Address = TxSigner::address(&signer);
        let mut wallet = EthereumWallet::from(signer.clone());
        wallet.register_signer(signer);

        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());

        // Fund the signer's address (Anvil starts with prefunded accounts)
        provider
            .anvil_set_balance(sender_address, U256::from(10_000_000_000_000_000u64))
            .await
            .unwrap();

        // Build a transaction to send 100 wei .
        let tx = TransactionRequest::default()
            .with_from(sender_address)
            .with_to(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
            .with_value(U256::from(100));

        // Send the transaction and wait for inclusion.
        let tx_hash = provider
            .send_transaction(tx)
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();

        println!("Sent transaction: {tx_hash}");

        cleanup_test_env();
    }

    #[tokio::test]
    async fn test_new_from_path() {
        let test_path = PathBuf::from("test_signer.stronghold");
        let signer = setup_test_env_with_path(test_path.clone(), Some(1));

        assert!(signer.address != Address::ZERO, "Address should be set");
        assert_eq!(signer.chain_id, Some(1), "Chain ID should match");

        // Verify the file was created
        assert!(test_path.exists(), "Stronghold file should exist");

        // Create a second signer with the same path to verify it loads the same key
        let signer2 = StrongholdSigner::new_from_path(test_path.clone(), Some(1))
            .expect("Failed to create second StrongholdSigner");
        assert_eq!(signer2.address, signer.address, "Should load same address");

        // Clean up
        fs::remove_file(&test_path).expect("Failed to remove test file");
        cleanup_test_env();
    }
}
