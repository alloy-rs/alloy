mod builder;

mod either;
use alloy_consensus::{Transaction as TxTrait, TxEnvelope};
pub use either::{AnyTxEnvelope, AnyTypedTransaction};

mod unknowns;
pub use unknowns::{AnyTxType, UnknownTxEnvelope, UnknownTypedTransaction};

pub use alloy_consensus_any::{AnyHeader, AnyReceiptEnvelope};

use crate::Network;
use alloy_network_primitives::BlockResponse;
pub use alloy_rpc_types_any::{AnyRpcHeader, AnyTransactionReceipt};
use alloy_rpc_types_eth::{Block, BlockTransactions, Transaction, TransactionRequest};
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
///   transaction types. It will succesfully decode an Ethereum [`TxEnvelope`], but will decode only
///   the type for any unknown transaction type. It will also leave the buffer unconsumed, which
///   will cause further deserialization to produce erroneous results.
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

    type TransactionResponse = WithOtherFields<Transaction<AnyTxEnvelope>>;

    type ReceiptResponse = AnyTransactionReceipt;

    type HeaderResponse = AnyRpcHeader;

    type BlockResponse = AnyRpcBlock;
}

/// A wrapper for [`AnyRpcBlock`] that allows for handling unknown block types.
#[derive(Clone, Debug, From, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnyRpcBlock(
    WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>,
);

impl AnyRpcBlock {
    /// Create a new [`AnyRpcBlock`].
    pub fn new(
        inner: WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>,
    ) -> Self {
        Self(inner)
    }

    /// Tries to convert inner transactions into a vector of [`AnyRpcTransaction`].
    ///
    /// Returns an error if the block contains only transaction hashes or if it is an uncle block.
    pub fn try_into_transactions(self) -> Result<Vec<AnyRpcTransaction>, String> {
        match self.0.inner.transactions {
            BlockTransactions::Full(txs) => {
                let mut result = Vec::with_capacity(txs.len());
                for tx in txs {
                    result.push(AnyRpcTransaction::new(tx));
                }
                Ok(result)
            }
            BlockTransactions::Hashes(_) => {
                Err("Block contains only transaction hashes".to_string())
            }
            BlockTransactions::Uncle => {
                Err("Block is an uncle block with no transactions".to_string())
            }
        }
    }
}

impl BlockResponse for AnyRpcBlock {
    type Header = AnyRpcHeader;
    type Transaction = WithOtherFields<Transaction<AnyTxEnvelope>>;

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

impl AsRef<WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>>
    for AnyRpcBlock
{
    fn as_ref(
        &self,
    ) -> &WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>> {
        &self.0
    }
}

impl Deref for AnyRpcBlock {
    type Target = WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>;

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
            .map_transactions(|tx| WithOtherFields::new(tx.map(AnyTxEnvelope::Ethereum)));

        Self(WithOtherFields::new(block))
    }
}

/// A wrapper for [`AnyRpcTransaction`] that allows for handling unknown transaction types.
#[derive(Clone, Debug, From, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnyRpcTransaction(WithOtherFields<Transaction<AnyTxEnvelope>>);

impl AnyRpcTransaction {
    /// Create a new [`AnyRpcTransaction`].
    pub fn new(inner: WithOtherFields<Transaction<AnyTxEnvelope>>) -> Self {
        Self(inner)
    }

    /// Split the transaction into its parts.
    pub fn into_parts(self) -> (Transaction<AnyTxEnvelope>, alloy_serde::OtherFields) {
        let WithOtherFields { inner, other } = self.0;
        (inner, other)
    }

    /// Returns the inner transaction [`TxEnvelope`] if inner tx type if
    /// [`AnyTxEnvelope::Ethereum`].
    pub fn as_envelope(self) -> Option<TxEnvelope> {
        let (tx, _other) = self.into_parts();
        tx.inner.as_envelope().cloned()
    }

    /// Maps the inner transaction to a new type that implements [`TxTrait`].
    ///
    /// [`alloy_serde::OtherFields`] are ignored while mapping.
    pub fn map<F, T: TxTrait>(self, f: F) -> T
    where
        F: FnOnce(Transaction<AnyTxEnvelope>) -> T,
    {
        let WithOtherFields { inner, other: _ } = self.0;
        f(inner)
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
