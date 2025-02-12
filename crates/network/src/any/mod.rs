mod builder;

mod either;
pub use either::{AnyTxEnvelope, AnyTypedTransaction};

mod unknowns;
pub use unknowns::{AnyTxType, UnknownTxEnvelope, UnknownTypedTransaction};

pub use alloy_consensus_any::{AnyHeader, AnyReceiptEnvelope};

use crate::Network;
pub use alloy_rpc_types_any::{AnyRpcHeader, AnyTransactionReceipt};
use alloy_rpc_types_eth::{Block, Transaction, TransactionRequest};
use alloy_serde::WithOtherFields;
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

    type BlockResponse =
        WithOtherFields<Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>>;
}

/// A wrapper for [`AnyRpcBlock`] that allows for handling unknown block types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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

impl<'de> Deserialize<'de> for AnyRpcBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = WithOtherFields::<
            Block<WithOtherFields<Transaction<AnyTxEnvelope>>, AnyRpcHeader>,
        >::deserialize(deserializer)?;
        Ok(Self(inner))
    }
}

/// A wrapper for [`AnyRpcTransaction`] that allows for handling unknown transaction types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnyRpcTransaction(WithOtherFields<Transaction<AnyTxEnvelope>>);

impl AnyRpcTransaction {
    /// Create a new [`AnyRpcTransaction`].
    pub fn new(inner: WithOtherFields<Transaction<AnyTxEnvelope>>) -> Self {
        Self(inner)
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

impl<'de> Deserialize<'de> for AnyRpcTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = WithOtherFields::<Transaction<AnyTxEnvelope>>::deserialize(deserializer)?;
        Ok(Self(inner))
    }
}
