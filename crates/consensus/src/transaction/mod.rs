use crate::Signed;
use alloy_primitives::{keccak256, ChainId, TxKind, B256, U256};
use core::any;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod eip1559;
pub use eip1559::TxEip1559;

mod eip2930;
pub use eip2930::TxEip2930;

mod eip4844;
#[cfg(feature = "kzg")]
pub use eip4844::BlobTransactionValidationError;
pub use eip4844::{
    utils as eip4844_utils, Blob, BlobTransactionSidecar, Bytes48, SidecarBuilder, SidecarCoder,
    SimpleCoder, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar,
};

mod envelope;
pub use envelope::{TxEnvelope, TxType};

mod legacy;
pub use legacy::TxLegacy;

mod typed;
pub use typed::TypedTransaction;

/// Represents a minimal EVM transaction.
pub trait Transaction: any::Any + Send + Sync + 'static {
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
    fn gas_limit(&self) -> u128;

    /// Get `gas_price`.
    fn gas_price(&self) -> Option<u128>;
}

/// A signable transaction.
///
/// A transaction can have multiple signature types. This is usually
/// [`alloy_primitives::Signature`], however, it may be different for future EIP-2718 transaction
/// types, or in other networks. For example, in Optimism, the deposit transaction signature is the
/// unit type `()`.
pub trait SignableTransaction<Signature>: Transaction {
    /// Sets `chain_id`.
    ///
    /// Prefer [`set_chain_id_checked`](Self::set_chain_id_checked).
    fn set_chain_id(&mut self, chain_id: ChainId);

    /// Set `chain_id` if it is not already set. Checks that the provided `chain_id` matches the
    /// existing `chain_id` if it is already set, returning `false` if they do not match.
    fn set_chain_id_checked(&mut self, chain_id: ChainId) -> bool {
        match self.chain_id() {
            Some(tx_chain_id) => {
                if tx_chain_id != chain_id {
                    return false;
                }
                self.set_chain_id(chain_id);
            }
            None => {
                self.set_chain_id(chain_id);
            }
        }
        true
    }

    /// RLP-encodes the transaction for signing.
    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut);

    /// Outputs the length of the signature RLP encoding for the transaction.
    fn payload_len_for_signature(&self) -> usize;

    /// RLP-encodes the transaction for signing it. Used to calculate `signature_hash`.
    ///
    /// See [`SignableTransaction::encode_for_signing`].
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
}

// TODO: Remove in favor of dyn trait upcasting (TBD, see https://github.com/rust-lang/rust/issues/65991#issuecomment-1903120162)
#[doc(hidden)]
impl<S: 'static> dyn SignableTransaction<S> {
    pub fn __downcast_ref<T: any::Any>(&self) -> Option<&T> {
        if any::Any::type_id(self) == any::TypeId::of::<T>() {
            unsafe { Some(&*(self as *const _ as *const T)) }
        } else {
            None
        }
    }
}
