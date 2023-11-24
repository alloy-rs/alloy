use crate::{Signature, Signer};
use alloy_primitives::{utils::eip191_hash_message, Address, B256};
use async_trait::async_trait;
use k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId};
use std::fmt;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

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
/// use alloy_signers::{LocalWallet, Signer};
///
/// let wallet = LocalWallet::random();
///
/// // Optionally, the wallet's chain id can be set, in order to use EIP-155
/// // replay protection with different chains
/// let wallet = wallet.with_chain_id(1337u64);
///
/// // The wallet can be used to sign messages
/// let message = b"hello";
/// let signature = wallet.sign_message(message)?;
/// assert_eq!(signature.recover_address_from_msg(&message[..]).unwrap(), wallet.address());
///
/// // LocalWallet is clonable:
/// let wallet_clone = wallet.clone();
/// let signature2 = wallet_clone.sign_message(message)?;
/// assert_eq!(signature, signature2);
/// # Ok::<_, Box<dyn std::error::Error>>(())
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)> + Send + Sync> Signer for Wallet<D> {
    type Error = WalletError;

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, Self::Error> {
        self.sign_message(message)
    }

    #[cfg(TODO)]
    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, Self::Error> {
        self.sign_transaction_sync(tx)
    }

    #[cfg(feature = "eip712")]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature, Self::Error> {
        self.sign_hash(&payload.eip712_signing_hash(domain))
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

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)> + Send + Sync> Wallet<D> {
    /// Construct a new wallet with an external Signer
    pub const fn new_with_signer(signer: D, address: Address, chain_id: u64) -> Self {
        Wallet { signer, address, chain_id }
    }

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

    /// Signs the provided message after prefixing it and hashing it according to [EIP-191].
    ///
    /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
    pub fn sign_message<T: AsRef<[u8]>>(&self, msg: T) -> Result<Signature, WalletError> {
        self.sign_hash(&eip191_hash_message(msg))
    }

    /// Signs the provided hash.
    pub fn sign_hash(&self, hash: &B256) -> Result<Signature, WalletError> {
        let (recoverable_sig, recovery_id) = self.signer.sign_prehash(hash.as_ref())?;
        Ok(Signature::new(recoverable_sig, recovery_id))
    }

    /// Returns this wallet's signer.
    pub const fn signer(&self) -> &D {
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
