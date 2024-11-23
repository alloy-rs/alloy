use super::UnknownTypedTransaction;

use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Bytes, ChainId, TxKind, B256, U256};

/// A transaction envelope from an unknown network.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "UnknownTransactionEnvelope")]
pub struct UnknownTxEnvelope {
    /// Transaction hash.
    pub hash: B256,

    /// Transaction type.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: UnknownTypedTransaction,
}

impl AsRef<UnknownTypedTransaction> for UnknownTxEnvelope {
    fn as_ref(&self) -> &UnknownTypedTransaction {
        &self.inner
    }
}

impl alloy_consensus::Transaction for UnknownTxEnvelope {
    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.inner.chain_id()
    }

    #[inline]
    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        self.inner.gas_price()
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        self.inner.max_fee_per_gas()
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.inner.max_priority_fee_per_gas()
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.inner.max_fee_per_blob_gas()
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.inner.priority_fee_or_price()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.inner.effective_gas_price(base_fee)
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        self.inner.is_dynamic_fee()
    }

    #[inline]
    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    #[inline]
    fn value(&self) -> U256 {
        self.inner.value()
    }

    #[inline]
    fn input(&self) -> &Bytes {
        self.inner.input()
    }

    #[inline]
    fn ty(&self) -> u8 {
        self.inner.ty()
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        self.inner.access_list()
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.inner.blob_versioned_hashes()
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.inner.authorization_list()
    }
}
