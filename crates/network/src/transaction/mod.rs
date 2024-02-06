use alloy_signer::LocalWallet;
use async_trait::async_trait;

use alloy_primitives::{keccak256, ChainId, Signature, B256, U256};
use alloy_rlp::BufMut;

mod builder;
pub use builder::{Builder, BuilderError, CanBuild};

mod common;
pub use common::TxKind;

mod signed;
pub use signed::Signed;

/// Transaction-like objects signable with a specific signature type.
pub trait Signable<Sig = Signature>: Transaction {
    /// RLP-encodes the transaction for signing.
    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut);

    /// Outputs the length of the signature RLP encoding for the transaction.
    fn payload_len_for_signature(&self) -> usize;

    /// RLP-encodes the transaction for signing it. Used to calculate `signature_hash`.
    ///
    /// See [`Transaction::encode_for_signing`].
    fn encoded_for_signing(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.payload_len_for_signature());
        self.encode_for_signing(&mut buf);
        buf
    }

    /// Calculate the signing hash for the transaction.
    fn signature_hash(&self) -> B256 {
        keccak256(self.encoded_for_signing())
    }

    /// Convert to a signed transaction by adding a signature and computing the
    /// hash.
    fn into_signed(self, signature: Sig) -> Signed<Self, Sig>
    where
        Self: Sized;

    /// Encode with a signature. This encoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    fn encode_signed(&self, signature: &Sig, out: &mut dyn BufMut);

    /// Decode a signed transaction. This decoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    ///
    /// This MUST be the inverse of [`Transaction::encode_signed`].
    fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self, Sig>>
    where
        Self: Sized;
}

/// A transaction signer.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TxSigner<Sig: 'static>: alloy_signer::Signer<Sig> {
    /// Signs the transaction.
    async fn sign_transaction(&self, tx: &mut dyn Signable<Sig>) -> alloy_signer::Result<Sig> {
        match (self.chain_id(), tx.chain_id()) {
            (Some(signer), Some(tx)) if signer != tx => {
                return Err(alloy_signer::Error::TransactionChainIdMismatch { signer, tx })
            }
            _ => {}
        }
        self.sign_hash(tx.signature_hash()).await
    }
}

impl TxSigner<Signature> for LocalWallet {}
impl TxSignerSync<Signature> for LocalWallet {}

/// A synchronous transaction signer.
pub trait TxSignerSync<Sig: 'static>: alloy_signer::SignerSync<Sig> {
    /// Signs the transaction.
    #[inline]
    fn sign_transaction_sync(&self, tx: &mut dyn Signable<Sig>) -> alloy_signer::Result<Sig> {
        match (self.chain_id_sync(), tx.chain_id()) {
            (Some(signer), Some(tx)) if signer != tx => {
                return Err(alloy_signer::Error::TransactionChainIdMismatch { signer, tx })
            }
            _ => {}
        }
        self.sign_hash_sync(tx.signature_hash())
    }
}

/// Represents a minimal EVM transaction.
pub trait Transaction: std::any::Any + Send + Sync + 'static {
    /// Get `data`.
    fn input(&self) -> &[u8];

    /// Get `to`.
    fn to(&self) -> TxKind;

    /// Get `value`.
    fn value(&self) -> U256;

    /// Get `chain_id`.
    fn chain_id(&self) -> Option<ChainId>;

    /// Get `nonce`.
    fn nonce(&self) -> u64;

    /// Get `gas_limit`.
    fn gas_limit(&self) -> u64;

    /// Get `gas_price`.
    fn gas_price(&self) -> Option<U256>;
}

// TODO: Remove in favor of dyn trait upcasting (TBD, see https://github.com/rust-lang/rust/issues/65991#issuecomment-1903120162)
#[doc(hidden)]
impl<S: 'static> dyn Signable<S> {
    pub fn __downcast_ref<T: std::any::Any>(&self) -> Option<&T> {
        if std::any::Any::type_id(self) == std::any::TypeId::of::<T>() {
            unsafe { Some(&*(self as *const _ as *const T)) }
        } else {
            None
        }
    }
}
