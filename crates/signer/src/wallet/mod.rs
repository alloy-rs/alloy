use crate::{Signature, Signer};
use alloy_primitives::{Address, B256};
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
/// then verified. Note that this uses
/// [`eip191_hash_message`](alloy_primitives::eip191_hash_message) under the hood which will
/// prefix the message being hashed with the `Ethereum Signed Message` domain separator.
///
/// ```
/// use alloy_signer::{LocalWallet, Signer};
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
pub struct Wallet<D> {
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

    #[inline]
    fn sign_hash(&self, hash: &B256) -> Result<Signature, Self::Error> {
        let (recoverable_sig, recovery_id) = self.signer.sign_prehash(hash.as_ref())?;
        Ok(Signature::new(recoverable_sig, recovery_id))
    }

    #[cfg(TODO)]
    #[inline]
    fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, Self::Error> {
        // rlp (for sighash) must have the same chain id as v in the signature
        let chain_id = tx.chain_id().map(|id| id.as_u64()).unwrap_or(self.chain_id);
        let mut tx = tx.clone();
        tx.set_chain_id(chain_id);

        let sighash = tx.sighash();
        let mut sig = self.sign_hash(&sighash)?;
        sig.apply_eip155(chain_id);
        Ok(sig)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: u64) {
        self.chain_id = chain_id;
    }
}

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)> + Send + Sync> Wallet<D> {
    /// Construct a new wallet with an external [`PrehashSigner`].
    #[inline]
    pub const fn new_with_signer(signer: D, address: Address, chain_id: u64) -> Self {
        Wallet { signer, address, chain_id }
    }

    /// Returns this wallet's signer.
    #[inline]
    pub const fn signer(&self) -> &D {
        &self.signer
    }
}

// do not log the signer
impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> fmt::Debug for Wallet<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}
