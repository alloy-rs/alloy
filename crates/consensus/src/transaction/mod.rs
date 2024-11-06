//! Transaction types.

use crate::Signed;
use alloc::vec::Vec;
use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{keccak256, Address, Bytes, ChainId, TxKind, B256, U256};
use core::{any, fmt};

mod eip1559;
pub use eip1559::TxEip1559;

mod eip2930;
pub use eip2930::TxEip2930;

mod eip7702;
pub use eip7702::TxEip7702;

/// [EIP-4844] constants, helpers, and types.
pub mod eip4844;

pub use alloy_eips::eip4844::{
    builder::{SidecarBuilder, SidecarCoder, SimpleCoder},
    utils as eip4844_utils, Blob, BlobTransactionSidecar, Bytes48,
};
#[cfg(feature = "kzg")]
pub use eip4844::BlobTransactionValidationError;
pub use eip4844::{TxEip4844, TxEip4844Variant, TxEip4844WithSidecar};

mod envelope;
pub use envelope::{TxEnvelope, TxType};

mod legacy;
pub use legacy::TxLegacy;

mod typed;
pub use typed::TypedTransaction;

/// Bincode-compatible serde implementations for transaction types.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub mod serde_bincode_compat {
    pub use super::{
        eip1559::serde_bincode_compat::*, eip2930::serde_bincode_compat::*,
        eip7702::serde_bincode_compat::*, legacy::serde_bincode_compat::*,
    };
}

/// Represents a minimal EVM transaction.
#[doc(alias = "Tx")]
pub trait Transaction: fmt::Debug + any::Any + Send + Sync + 'static {
    /// Get `chain_id`.
    fn chain_id(&self) -> Option<ChainId>;

    /// Get `nonce`.
    fn nonce(&self) -> u64;

    /// Get `gas_limit`.
    fn gas_limit(&self) -> u64;

    /// Get `gas_price`.
    fn gas_price(&self) -> Option<u128>;

    /// Returns the EIP-1559 the maximum fee per gas the caller is willing to pay.
    ///
    /// For legacy transactions this is `gas_price`.
    ///
    /// This is also commonly referred to as the "Gas Fee Cap".
    fn max_fee_per_gas(&self) -> u128;

    /// Returns the EIP-1559 Priority fee the caller is paying to the block author.
    ///
    /// This will return `None` for non-EIP1559 transactions
    fn max_priority_fee_per_gas(&self) -> Option<u128>;

    /// Max fee per blob gas for EIP-4844 transaction.
    ///
    /// Returns `None` for non-eip4844 transactions.
    ///
    /// This is also commonly referred to as the "Blob Gas Fee Cap".
    fn max_fee_per_blob_gas(&self) -> Option<u128>;

    /// Return the max priority fee per gas if the transaction is an EIP-1559 transaction, and
    /// otherwise return the gas price.
    ///
    /// # Warning
    ///
    /// This is different than the `max_priority_fee_per_gas` method, which returns `None` for
    /// non-EIP-1559 transactions.
    fn priority_fee_or_price(&self) -> u128;

    /// Returns the effective tip for this transaction.
    ///
    /// For EIP-1559 transactions: `min(max_fee_per_gas - base_fee, max_priority_fee_per_gas)`.
    /// For legacy transactions: `gas_price - base_fee`.
    fn effective_tip_per_gas(&self, base_fee: u64) -> Option<u128> {
        let base_fee = base_fee as u128;

        let max_fee_per_gas = self.max_fee_per_gas();

        // Check if max_fee_per_gas is less than base_fee
        if max_fee_per_gas < base_fee {
            return None;
        }

        // Calculate the difference between max_fee_per_gas and base_fee
        let fee = max_fee_per_gas - base_fee;

        // Compare the fee with max_priority_fee_per_gas (or gas price for non-EIP1559 transactions)
        self.max_priority_fee_per_gas()
            .map_or(Some(fee), |priority_fee| Some(fee.min(priority_fee)))
    }

    /// Returns the transaction kind.
    fn kind(&self) -> TxKind;

    /// Get the transaction's address of the contract that will be called, or the address that will
    /// receive the transfer.
    ///
    /// Returns `None` if this is a `CREATE` transaction.
    fn to(&self) -> Option<Address> {
        self.kind().to().copied()
    }

    /// Get `value`.
    fn value(&self) -> U256;

    /// Get `data`.
    fn input(&self) -> &Bytes;

    /// Returns the transaction type
    fn ty(&self) -> u8;

    /// Returns the EIP-2930 `access_list` for the particular transaction type. Returns `None` for
    /// older transaction types.
    fn access_list(&self) -> Option<&AccessList>;

    /// Blob versioned hashes for eip4844 transaction. For previous transaction types this is
    /// `None`.
    fn blob_versioned_hashes(&self) -> Option<&[B256]>;

    /// Returns the [`SignedAuthorization`] list of the transaction.
    ///
    /// Returns `None` if this transaction is not EIP-7702.
    fn authorization_list(&self) -> Option<&[SignedAuthorization]>;
}

/// A signable transaction.
///
/// A transaction can have multiple signature types. This is usually
/// [`alloy_primitives::Signature`], however, it may be different for future EIP-2718 transaction
/// types, or in other networks. For example, in Optimism, the deposit transaction signature is the
/// unit type `()`.
#[doc(alias = "SignableTx", alias = "TxSignable")]
pub trait SignableTransaction<Signature>: Transaction {
    /// True if the transaction uses EIP-155 signatures.
    fn use_eip155(&self) -> bool {
        false
    }

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

#[cfg(feature = "serde")]
impl<T: Transaction> Transaction for alloy_serde::WithOtherFields<T> {
    fn chain_id(&self) -> Option<ChainId> {
        self.inner.chain_id()
    }

    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.inner.gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.inner.max_fee_per_gas()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.inner.max_priority_fee_per_gas()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.inner.max_fee_per_blob_gas()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.inner.priority_fee_or_price()
    }

    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    fn value(&self) -> U256 {
        self.inner.value()
    }

    fn input(&self) -> &Bytes {
        self.inner.input()
    }

    fn ty(&self) -> u8 {
        self.inner.ty()
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.inner.access_list()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.inner.blob_versioned_hashes()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.inner.authorization_list()
    }
}
