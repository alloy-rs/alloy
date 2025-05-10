mod builder;

mod either;
use alloy_consensus::TxEnvelope;
use alloy_eips::{eip7702::SignedAuthorization, Typed2718};
use alloy_primitives::{Bytes, ChainId, TxKind, B256, U256};
pub use either::{AnyTxEnvelope, AnyTypedTransaction};

mod unknowns;
pub use unknowns::{AnyTxType, UnknownTxEnvelope, UnknownTypedTransaction};

pub use alloy_consensus_any::{AnyHeader, AnyReceiptEnvelope};

use crate::Network;
use alloy_consensus::{
    error::ValueError,
    transaction::{Either, Recovered},
};
use alloy_network_primitives::{BlockResponse, TransactionResponse};
pub use alloy_rpc_types_any::{AnyRpcHeader, AnyTransactionReceipt};
use alloy_rpc_types_eth::{AccessList, Block, BlockTransactions, Transaction, TransactionRequest};
use alloy_serde::WithOtherFields;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Types for a catch-all network.
///
/// `AnyNetwork`'s associated types allow for many different types of
/// transactions, using catch-all fields. This [`Network`] should be used
/// only when the application needs to support multiple networks via the same
/// codepaths without knowing the networks at compile time.
///
/// ## Rough Edges
///
/// Supporting arbitrary unknown types is hard, and users of this network
/// should be aware of the following:
///
/// - The implementation of [`Decodable2718`] for [`AnyTxEnvelope`] will not work for non-Ethereum
///   transaction types. It will successfully decode an Ethereum [`TxEnvelope`], but will decode
///   only the type for any unknown transaction type. It will also leave the buffer unconsumed,
///   which will cause further deserialization to produce erroneous results.
/// - The implementation of [`Encodable2718`] for [`AnyTypedTransaction`] will not work for
///   non-Ethereum transaction types. It will encode the type for any unknown transaction type, but
///   will not encode any other fields. This is symmetric with the decoding behavior, but still
///   erroneous.
/// - The [`TransactionRequest`] will build ONLY Ethereum types. It will error when attempting to
///   build any unknown type.
/// - The [`Network::TransactionResponse`] may deserialize unknown metadata fields into the inner
///   [`AnyTxEnvelope`], rather than into the outer [`WithOtherFields`].
///
/// [`Decodable2718`]: alloy_eips::eip2718::Decodable2718
/// [`Encodable2718`]: alloy_eips::eip2718::Encodable2718
/// [`TxEnvelope`]: alloy_consensus::TxEnvelope
#[derive(Clone, Copy, Debug)]
pub struct AnyNetwork {
    _private: (),
}

impl Network for AnyNetwork {
    type TxType = AnyTxType;

    type TxEnvelope = AnyTxEnvelope;

    type UnsignedTx = AnyTypedTransaction;

    type ReceiptEnvelope = AnyReceiptEnvelope;

    type Header = AnyHeader;

    type TransactionRequest = WithOtherFields<TransactionRequest>;

    type TransactionResponse = AnyRpcTransaction;

    type ReceiptResponse = AnyTransactionReceipt;

    type HeaderResponse = AnyRpcHeader;

    type BlockResponse = AnyRpcBlock;
}

/// A wrapper for [`AnyRpcBlock`] that allows for handling unknown block types.
///
/// This type wraps:
///  - rpc transaction
///  - additional fields
#[derive(Clone, Debug, From, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnyRpcBlock(pub WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>>);

impl AnyRpcBlock {
    /// Create a new [`AnyRpcBlock`].
    pub const fn new(inner: WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>>) -> Self {
        Self(inner)
    }

    /// Consumes the type and returns the wrapped rpc block.
    pub fn into_inner(self) -> Block<AnyRpcTransaction, AnyRpcHeader> {
        self.0.into_inner()
    }

    /// Tries to convert inner transactions into a vector of [`AnyRpcTransaction`].
    ///
    /// Returns an error if the block contains only transaction hashes or if it is an uncle block.
    pub fn try_into_transactions(
        self,
    ) -> Result<Vec<AnyRpcTransaction>, ValueError<BlockTransactions<AnyRpcTransaction>>> {
        self.0.inner.try_into_transactions()
    }

    /// Consumes the type and returns an iterator over the transactions in this block
    pub fn into_transactions_iter(self) -> impl Iterator<Item = AnyRpcTransaction> {
        self.into_inner().transactions.into_transactions()
    }
}

impl BlockResponse for AnyRpcBlock {
    type Header = AnyRpcHeader;
    type Transaction = AnyRpcTransaction;

    fn header(&self) -> &Self::Header {
        &self.0.inner.header
    }

    fn transactions(&self) -> &BlockTransactions<Self::Transaction> {
        &self.0.inner.transactions
    }

    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction> {
        &mut self.0.inner.transactions
    }

    fn other_fields(&self) -> Option<&alloy_serde::OtherFields> {
        self.0.other_fields()
    }
}

impl AsRef<WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>>> for AnyRpcBlock {
    fn as_ref(&self) -> &WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>> {
        &self.0
    }
}

impl Deref for AnyRpcBlock {
    type Target = WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AnyRpcBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Block> for AnyRpcBlock {
    fn from(value: Block) -> Self {
        let block = value
            .map_header(|h| h.map(|h| alloy_consensus_any::AnyHeader { ..h.into() }))
            .map_transactions(|tx| {
                AnyRpcTransaction::new(WithOtherFields::new(tx.map(AnyTxEnvelope::Ethereum)))
            });

        Self(WithOtherFields::new(block))
    }
}

impl From<AnyRpcBlock> for Block<AnyRpcTransaction, AnyRpcHeader> {
    fn from(value: AnyRpcBlock) -> Self {
        value.into_inner()
    }
}
impl From<AnyRpcBlock> for WithOtherFields<Block<AnyRpcTransaction, AnyRpcHeader>> {
    fn from(value: AnyRpcBlock) -> Self {
        value.0
    }
}

/// A wrapper for [`AnyRpcTransaction`] that allows for handling unknown transaction types.
#[derive(Clone, Debug, From, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnyRpcTransaction(pub WithOtherFields<Transaction<AnyTxEnvelope>>);

impl AnyRpcTransaction {
    /// Create a new [`AnyRpcTransaction`].
    pub const fn new(inner: WithOtherFields<Transaction<AnyTxEnvelope>>) -> Self {
        Self(inner)
    }

    /// Split the transaction into its parts.
    pub fn into_parts(self) -> (Transaction<AnyTxEnvelope>, alloy_serde::OtherFields) {
        let WithOtherFields { inner, other } = self.0;
        (inner, other)
    }

    /// Consumes the outer layer for this transaction and returns the inner transaction.
    pub fn into_inner(self) -> Transaction<AnyTxEnvelope> {
        self.0.into_inner()
    }

    /// Returns the inner transaction [`TxEnvelope`] if inner tx type if
    /// [`AnyTxEnvelope::Ethereum`].
    pub fn as_envelope(&self) -> Option<&TxEnvelope> {
        self.inner.inner.as_envelope()
    }

    /// Returns the inner Ethereum transaction envelope, if it is an Ethereum transaction.
    /// If the transaction is not an Ethereum transaction, it is returned as an error.
    pub fn try_into_envelope(self) -> Result<TxEnvelope, ValueError<AnyTxEnvelope>> {
        self.0.inner.inner.into_inner().try_into_envelope()
    }

    /// Attempts to convert the [`AnyRpcTransaction`] into `Either::Right` if this is an unknown
    /// variant.
    ///
    /// Returns `Either::Left` with the ethereum `TxEnvelope` if this is the
    /// [`AnyTxEnvelope::Ethereum`] variant and [`Either::Right`] with the converted variant.
    pub fn try_into_either<T>(self) -> Result<Either<TxEnvelope, T>, T::Error>
    where
        T: TryFrom<Self>,
    {
        if self.0.inner.inner.inner().is_ethereum() {
            Ok(Either::Left(self.0.inner.inner.into_inner().try_into_envelope().unwrap()))
        } else {
            T::try_from(self).map(Either::Right)
        }
    }

    /// Attempts to convert the [`UnknownTxEnvelope`] into `Either::Right` if this is an unknown
    /// variant.
    ///
    /// Returns `Either::Left` with the ethereum `TxEnvelope` if this is the
    /// [`AnyTxEnvelope::Ethereum`] variant and [`Either::Right`] with the converted variant.
    pub fn try_unknown_into_either<T>(self) -> Result<Either<TxEnvelope, T>, T::Error>
    where
        T: TryFrom<UnknownTxEnvelope>,
    {
        self.0.inner.inner.into_inner().try_into_either()
    }

    /// Applies the given closure to the inner transaction type.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    /// Applies the given closure to the inner transaction type.
    pub fn map<Tx>(self, f: impl FnOnce(AnyTxEnvelope) -> Tx) -> Transaction<Tx> {
        self.into_inner().map(f)
    }

    /// Applies the given fallible closure to the inner transactions.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    pub fn try_map<Tx, E>(
        self,
        f: impl FnOnce(AnyTxEnvelope) -> Result<Tx, E>,
    ) -> Result<Transaction<Tx>, E> {
        self.into_inner().try_map(f)
    }

    /// Converts the transaction type to the given alternative that is `From<T>`.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    pub fn convert<U>(self) -> Transaction<U>
    where
        U: From<AnyTxEnvelope>,
    {
        self.into_inner().map(U::from)
    }

    /// Converts the transaction to the given alternative that is `TryFrom<T>`
    ///
    /// Returns the transaction with the new transaction type if all conversions were successful.
    ///
    /// [`alloy_serde::OtherFields`] are stripped away while mapping.
    pub fn try_convert<U>(self) -> Result<Transaction<U>, U::Error>
    where
        U: TryFrom<AnyTxEnvelope>,
    {
        self.into_inner().try_map(U::try_from)
    }
}

impl AsRef<AnyTxEnvelope> for AnyRpcTransaction {
    fn as_ref(&self) -> &AnyTxEnvelope {
        &self.0.inner.inner
    }
}

impl Deref for AnyRpcTransaction {
    type Target = WithOtherFields<Transaction<AnyTxEnvelope>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AnyRpcTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Transaction<TxEnvelope>> for AnyRpcTransaction {
    fn from(tx: Transaction<TxEnvelope>) -> Self {
        let tx = tx.map(AnyTxEnvelope::Ethereum);
        Self(WithOtherFields::new(tx))
    }
}

impl From<AnyRpcTransaction> for AnyTxEnvelope {
    fn from(tx: AnyRpcTransaction) -> Self {
        tx.0.inner.into_inner()
    }
}

impl From<AnyRpcTransaction> for Transaction<AnyTxEnvelope> {
    fn from(tx: AnyRpcTransaction) -> Self {
        tx.0.inner
    }
}

impl From<AnyRpcTransaction> for WithOtherFields<Transaction<AnyTxEnvelope>> {
    fn from(tx: AnyRpcTransaction) -> Self {
        tx.0
    }
}

impl From<AnyRpcTransaction> for Recovered<AnyTxEnvelope> {
    fn from(tx: AnyRpcTransaction) -> Self {
        tx.0.inner.inner
    }
}

impl TryFrom<AnyRpcTransaction> for TxEnvelope {
    type Error = ValueError<AnyTxEnvelope>;

    fn try_from(value: AnyRpcTransaction) -> Result<Self, Self::Error> {
        value.try_into_envelope()
    }
}

impl alloy_consensus::Transaction for AnyRpcTransaction {
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
        alloy_consensus::Transaction::gas_price(&self.0.inner)
    }

    fn max_fee_per_gas(&self) -> u128 {
        alloy_consensus::Transaction::max_fee_per_gas(&self.inner)
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

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.inner.effective_gas_price(base_fee)
    }

    fn is_dynamic_fee(&self) -> bool {
        self.inner.is_dynamic_fee()
    }

    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    fn is_create(&self) -> bool {
        self.inner.is_create()
    }

    fn value(&self) -> U256 {
        self.inner.value()
    }

    fn input(&self) -> &Bytes {
        self.inner.input()
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

impl TransactionResponse for AnyRpcTransaction {
    fn tx_hash(&self) -> alloy_primitives::TxHash {
        self.inner.tx_hash()
    }

    fn block_hash(&self) -> Option<alloy_primitives::BlockHash> {
        self.0.inner.block_hash
    }

    fn block_number(&self) -> Option<u64> {
        self.inner.block_number
    }

    fn transaction_index(&self) -> Option<u64> {
        self.inner.transaction_index
    }

    fn from(&self) -> alloy_primitives::Address {
        self.inner.from()
    }

    fn gas_price(&self) -> Option<u128> {
        self.inner.effective_gas_price
    }
}

impl Typed2718 for AnyRpcTransaction {
    fn ty(&self) -> u8 {
        self.inner.ty()
    }
}
