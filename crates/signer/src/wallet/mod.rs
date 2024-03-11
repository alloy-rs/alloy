//! In-memory (local) private key signer.

use crate::{Result, Signer, SignerSync};
use alloy_primitives::{Address, ChainId, Signature, B256};
use async_trait::async_trait;
use k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId};
use std::fmt;

mod error;
pub use error::WalletError;

#[cfg(feature = "mnemonic")]
mod mnemonic;
#[cfg(feature = "mnemonic")]
pub use mnemonic::MnemonicBuilder;

mod private_key;

#[cfg(feature = "yubihsm")]
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
/// use alloy_signer::{LocalWallet, Signer, SignerSync};
///
/// let wallet = LocalWallet::random();
///
/// // Optionally, the wallet's chain id can be set, in order to use EIP-155
/// // replay protection with different chains
/// let wallet = wallet.with_chain_id(Some(1337));
///
/// // The wallet can be used to sign messages
/// let message = b"hello";
/// let signature = wallet.sign_message_sync(message)?;
/// assert_eq!(signature.recover_address_from_msg(&message[..]).unwrap(), wallet.address());
///
/// // LocalWallet is clonable:
/// let wallet_clone = wallet.clone();
/// let signature2 = wallet_clone.sign_message_sync(message)?;
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
    pub(crate) chain_id: Option<ChainId>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)> + Send + Sync> Signer for Wallet<D> {
    #[inline]
    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        self.sign_hash_sync(hash)
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

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> SignerSync for Wallet<D> {
    #[inline]
    fn sign_hash_sync(&self, hash: &B256) -> Result<Signature> {
        let (recoverable_sig, recovery_id) = self.signer.sign_prehash(hash.as_ref())?;
        let mut sig = Signature::from_signature_and_parity(recoverable_sig, recovery_id)?;
        if let Some(chain_id) = self.chain_id {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
    }

    #[inline]
    fn chain_id_sync(&self) -> Option<ChainId> {
        self.chain_id
    }
}

impl<D: PrehashSigner<(ecdsa::Signature, RecoveryId)>> Wallet<D> {
    /// Construct a new wallet with an external [`PrehashSigner`].
    #[inline]
    pub const fn new_with_signer(signer: D, address: Address, chain_id: Option<ChainId>) -> Self {
        Wallet { signer, address, chain_id }
    }

    /// Returns this wallet's signer.
    #[inline]
    pub const fn signer(&self) -> &D {
        &self.signer
    }

    /// Consumes this wallet and returns its signer.
    #[inline]
    pub fn into_signer(self) -> D {
        self.signer
    }

    /// Returns this wallet's chain ID.
    #[inline]
    pub const fn address(&self) -> Address {
        self.address
    }

    /// Returns this wallet's chain ID.
    #[inline]
    pub const fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
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
