use alloy_network_primitives::ReceiptResponse;
use alloy_rpc_types_eth::{Log, TransactionReceipt};
use alloy_serde::WithOtherFields;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::AnyReceiptEnvelope;

/// A wrapper for [`AnyTransactionReceipt`] that allows for handling unknown receipt types.
#[derive(Clone, Debug, From, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnyTransactionReceipt(pub WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>);

impl AnyTransactionReceipt {
    /// Create a new [`AnyTransactionReceipt`].
    pub const fn new(inner: WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>) -> Self {
        Self(inner)
    }

    /// Split the receipt into its parts.
    pub fn into_parts(
        self,
    ) -> (TransactionReceipt<AnyReceiptEnvelope<Log>>, alloy_serde::OtherFields) {
        let WithOtherFields { inner, other } = self.0;
        (inner, other)
    }

    /// Consumes the outer layer for this receipt and returns the inner receipt.
    pub fn into_inner(self) -> TransactionReceipt<AnyReceiptEnvelope<Log>> {
        self.0.into_inner()
    }

    /// Returns true if the transaction was successful.
    #[inline]
    pub fn is_success(&self) -> bool {
        self.0.inner.status()
    }

    /// Returns the contract address if this was a deployment transaction.
    #[inline]
    pub const fn deployed_contract(&self) -> Option<alloy_primitives::Address> {
        self.0.inner.contract_address
    }

    /// Returns the transaction hash.
    #[inline]
    pub const fn transaction_hash(&self) -> alloy_primitives::TxHash {
        self.0.inner.transaction_hash
    }

    /// Returns the transaction hash.
    ///
    /// Alias for [`transaction_hash`](Self::transaction_hash).
    #[inline]
    pub const fn tx_hash(&self) -> alloy_primitives::TxHash {
        self.transaction_hash()
    }

    /// Returns the logs from this receipt.
    #[inline]
    pub fn logs(&self) -> &[Log] {
        self.0.inner.logs()
    }

    /// Returns the block hash if available.
    #[inline]
    pub const fn block_hash(&self) -> Option<alloy_primitives::BlockHash> {
        self.0.inner.block_hash
    }

    /// Returns the block number if available.
    #[inline]
    pub const fn block_number(&self) -> Option<u64> {
        self.0.inner.block_number
    }

    /// Returns the gas used by this transaction.
    #[inline]
    pub const fn gas_used(&self) -> u64 {
        self.0.inner.gas_used
    }

    /// Returns the cumulative gas used up to this transaction in the block.
    #[inline]
    pub fn cumulative_gas_used(&self) -> u64 {
        self.0.inner.cumulative_gas_used()
    }

    /// Returns the effective gas price.
    #[inline]
    pub const fn effective_gas_price(&self) -> u128 {
        self.0.inner.effective_gas_price
    }

    /// Returns a reference to the other fields.
    #[inline]
    pub const fn other_fields(&self) -> &alloy_serde::OtherFields {
        &self.0.other
    }

    /// Returns a mutable reference to the other fields.
    #[inline]
    pub const fn other_fields_mut(&mut self) -> &mut alloy_serde::OtherFields {
        &mut self.0.other
    }

    /// Deserialize the other fields into a specific type.
    #[inline]
    pub fn deserialize_other<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.other.clone().deserialize_into()
    }

    /// Maps the inner receipt envelope to a different type.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    #[inline]
    pub fn map_inner<U, F>(self, f: F) -> TransactionReceipt<U>
    where
        F: FnOnce(AnyReceiptEnvelope<Log>) -> U,
    {
        let WithOtherFields { inner, .. } = self.0;
        inner.map_inner(f)
    }

    /// Applies a fallible mapping function to the inner receipt envelope.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    #[inline]
    pub fn try_map_inner<U, E, F>(self, f: F) -> Result<TransactionReceipt<U>, E>
    where
        F: FnOnce(AnyReceiptEnvelope<Log>) -> Result<U, E>,
    {
        let WithOtherFields { inner, .. } = self.0;
        Ok(TransactionReceipt {
            inner: f(inner.inner)?,
            transaction_hash: inner.transaction_hash,
            transaction_index: inner.transaction_index,
            block_hash: inner.block_hash,
            block_number: inner.block_number,
            gas_used: inner.gas_used,
            effective_gas_price: inner.effective_gas_price,
            blob_gas_used: inner.blob_gas_used,
            blob_gas_price: inner.blob_gas_price,
            from: inner.from,
            to: inner.to,
            contract_address: inner.contract_address,
        })
    }

    /// Converts the receipt to a different envelope type.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    #[inline]
    pub fn convert<U>(self) -> TransactionReceipt<U>
    where
        U: From<AnyReceiptEnvelope<Log>>,
    {
        let WithOtherFields { inner, .. } = self.0;
        inner.map_inner(U::from)
    }

    /// Tries to convert the receipt to a different envelope type.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    #[inline]
    pub fn try_convert<U>(
        self,
    ) -> Result<TransactionReceipt<U>, <U as TryFrom<AnyReceiptEnvelope<Log>>>::Error>
    where
        U: TryFrom<AnyReceiptEnvelope<Log>>,
    {
        self.try_map_inner(U::try_from)
    }
}

impl Deref for AnyTransactionReceipt {
    type Target = WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AnyTransactionReceipt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<TransactionReceipt<AnyReceiptEnvelope<Log>>> for AnyTransactionReceipt {
    fn from(receipt: TransactionReceipt<AnyReceiptEnvelope<Log>>) -> Self {
        Self(WithOtherFields::new(receipt))
    }
}

impl From<AnyTransactionReceipt> for TransactionReceipt<AnyReceiptEnvelope<Log>> {
    fn from(receipt: AnyTransactionReceipt) -> Self {
        receipt.0.inner
    }
}

impl From<AnyTransactionReceipt> for WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>> {
    fn from(receipt: AnyTransactionReceipt) -> Self {
        receipt.0
    }
}

impl ReceiptResponse for AnyTransactionReceipt {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.0.inner.contract_address
    }

    fn status(&self) -> bool {
        self.0.inner.status()
    }

    fn block_hash(&self) -> Option<alloy_primitives::BlockHash> {
        self.0.inner.block_hash
    }

    fn block_number(&self) -> Option<u64> {
        self.0.inner.block_number
    }

    fn transaction_hash(&self) -> alloy_primitives::TxHash {
        self.0.inner.transaction_hash
    }

    fn transaction_index(&self) -> Option<u64> {
        self.0.inner.transaction_index
    }

    fn gas_used(&self) -> u64 {
        self.0.inner.gas_used
    }

    fn effective_gas_price(&self) -> u128 {
        self.0.inner.effective_gas_price
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.0.inner.blob_gas_used
    }

    fn blob_gas_price(&self) -> Option<u128> {
        self.0.inner.blob_gas_price
    }

    fn from(&self) -> alloy_primitives::Address {
        self.0.inner.from
    }

    fn to(&self) -> Option<alloy_primitives::Address> {
        self.0.inner.to
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.0.inner.cumulative_gas_used()
    }

    fn state_root(&self) -> Option<alloy_primitives::B256> {
        self.0.inner.state_root()
    }
}
