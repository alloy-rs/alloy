use crate::{Network, Signed, Transaction};
use alloy_primitives::{keccak256, B256};
use alloy_rlp::BufMut;
use async_trait::async_trait;

/// A signable transaction.
///
/// A transaction can have multiple signature types. This is usually
/// [`alloy_primitives::Signature`], however, it may be different for future EIP-2718 transaction
/// types, or in other networks. For example, in Optimism, the deposit transaction signature is the
/// unit type `()`.
pub trait SignableTransaction<Signature>: Transaction {
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
    fn into_signed(self, signature: Signature) -> Signed<Self, Signature>
    where
        Self: Sized;

    /// Encode with a signature. This encoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    fn encode_signed(&self, signature: &Signature, out: &mut dyn BufMut);

    /// Decode a signed transaction. This decoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    ///
    /// This MUST be the inverse of [`Transaction::encode_signed`].
    fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self, Signature>>
    where
        Self: Sized;
}

// TODO: Remove in favor of dyn trait upcasting (TBD, see https://github.com/rust-lang/rust/issues/65991#issuecomment-1903120162)
#[doc(hidden)]
impl<S: 'static> dyn SignableTransaction<S> {
    pub fn __downcast_ref<T: std::any::Any>(&self) -> Option<&T> {
        if std::any::Any::type_id(self) == std::any::TypeId::of::<T>() {
            unsafe { Some(&*(self as *const _ as *const T)) }
        } else {
            None
        }
    }
}

// todo: move
/// A signer capable of signing any transaction for the given network.
#[async_trait]
pub trait NetworkSigner<N: Network> {
    /// Asynchronously sign an unsigned transaction.
    async fn sign(&self, tx: N::UnsignedTx) -> alloy_signer::Result<N::TxEnvelope>;
}

// todo: move
/// An async signer capable of signing any [SignableTransaction] for the given [Signature] type.
#[async_trait]
pub trait TxSigner<Signature> {
    /// Asynchronously sign an unsigned transaction.
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}

// todo: move
/// A sync signer capable of signing any [SignableTransaction] for the given [Signature] type.
pub trait TxSignerSync<Signature> {
    /// Synchronously sign an unsigned transaction.
    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature>;
}
