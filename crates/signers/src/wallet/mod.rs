use crate::{Signature, Signer};
use alloy_primitives::{utils::eip191_hash_message, Address, B256};
use async_trait::async_trait;
use k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId};
use std::fmt;

mod mnemonic;
pub use mnemonic::MnemonicBuilder;

mod private_key;
pub use private_key::WalletError;

#[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
mod yubi;

/// An Ethereum private-public key pair which can be used for signing messages.
///
/// # Examples
///
/// ## Signing and Verifying a message
///
/// The wallet can be used to produce ECDSA [`Signature`] objects, which can be
/// then verified. Note that this uses [`eip191_hash_message`] under the hood which will
/// prefix the message being hashed with the `Ethereum Signed Message` domain separator.
///
/// ```
/// use ethers_core::rand::thread_rng;
/// use ethers_signers::{LocalWallet, Signer};
///
/// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
/// let wallet = LocalWallet::new(&mut thread_rng());
///
/// // Optionally, the wallet's chain id can be set, in order to use EIP-155
/// // replay protection with different chains
/// let wallet = wallet.with_chain_id(1337u64);
///
/// // The wallet can be used to sign messages
/// let message = b"hello";
/// let signature = wallet.sign_message(message).await?;
/// assert_eq!(signature.recover(&message[..]).unwrap(), wallet.address());
///
/// // LocalWallet is clonable:
/// let wallet_clone = wallet.clone();
/// let signature2 = wallet_clone.sign_message(message).await?;
/// assert_eq!(signature, signature2);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Wallet<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> {
    /// The wallet's private key.
    pub(crate) signer: D,
    /// The wallet's address.
    pub(crate) address: Address,
    /// The wallet's chain ID (for EIP-155).
    pub(crate) chain_id: u64,
}

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> Wallet<D> {
    /// Construct a new wallet with an external Signer
    pub fn new_with_signer(signer: D, address: Address, chain_id: u64) -> Self {
        Wallet { signer, address, chain_id }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<D: Sync + Send + PrehashSigner<(ecdsa::Signature, RecoveryId)>> Signer for Wallet<D> {
    type Error = WalletError;

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, Self::Error> {
        let message_hash = eip191_hash_message(message);
        self.sign_hash(message_hash)
    }

    #[cfg(TODO)]
    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, Self::Error> {
        self.sign_transaction_sync(tx)
    }

    #[cfg(TODO)]
    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error> {
        let encoded =
            payload.encode_eip712().map_err(|e| Self::Error::Eip712Error(e.to_string()))?;

        self.sign_hash(B256::from(encoded))
    }

    fn address(&self) -> Address {
        self.address
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.chain_id = chain_id.into();
        self
    }
}

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> Wallet<D> {
    /// Synchronously signs the provided transaction, normalizing the signature `v` value with
    /// EIP-155 using the transaction's `chain_id`, or the signer's `chain_id` if the transaction
    /// does not specify one.
    #[cfg(TODO)]
    pub fn sign_transaction_sync(&self, tx: &TypedTransaction) -> Result<Signature, WalletError> {
        // rlp (for sighash) must have the same chain id as v in the signature
        let chain_id = tx.chain_id().map(|id| id.as_u64()).unwrap_or(self.chain_id);
        let mut tx = tx.clone();
        tx.set_chain_id(chain_id);

        let sighash = tx.sighash();
        let mut sig = self.sign_hash(sighash)?;
        sig.set_v(to_eip155_v(sig.recid().to_byte(), chain_id));
        Ok(sig)
    }

    /// Signs the provided hash.
    pub fn sign_hash(&self, hash: B256) -> Result<Signature, WalletError> {
        let (recoverable_sig, recovery_id) = self.signer.sign_prehash(hash.as_ref())?;
        Ok(Signature::new(recoverable_sig, recovery_id))
    }

    /// Gets the wallet's signer
    pub fn signer(&self) -> &D {
        &self.signer
    }
}

// do not log the signer
impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> fmt::Debug for Wallet<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .field("chain_Id", &self.chain_id)
            .finish()
    }
}
