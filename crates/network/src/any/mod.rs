use core::fmt;

use crate::{Network, ReceiptResponse, TransactionResponse};
use alloy_consensus::TxType;
use alloy_eips::eip2718::Eip2718Error;
use alloy_rpc_types_eth::{
    AnyTransactionReceipt, Header, Transaction, TransactionRequest, WithOtherFields,
};

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(alias = "AnyTransactionType")]
pub struct AnyTxType(u8);

impl fmt::Display for AnyTxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyTxType({})", self.0)
    }
}

impl TryFrom<u8> for AnyTxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<AnyTxType> for u8 {
    fn from(value: AnyTxType) -> Self {
        value.0
    }
}

impl TryFrom<AnyTxType> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: AnyTxType) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl From<TxType> for AnyTxType {
    fn from(value: TxType) -> Self {
        Self(value as u8)
    }
}

/// Types for a catch-all network.
///
/// Essentially just returns the regular Ethereum types + a catch all field.
/// This [`Network`] should be used only when the network is not known at
/// compile time.
#[derive(Clone, Copy, Debug)]
pub struct AnyNetwork {
    _private: (),
}

impl Network for AnyNetwork {
    type TxType = AnyTxType;

    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::AnyReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = WithOtherFields<TransactionRequest>;

    type TransactionResponse = WithOtherFields<Transaction>;

    type ReceiptResponse = AnyTransactionReceipt;

    type HeaderResponse = WithOtherFields<Header>;
}

impl ReceiptResponse for AnyTransactionReceipt {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }
}

impl TransactionResponse for WithOtherFields<Transaction> {
    #[doc(alias = "transaction_hash")]
    fn tx_hash(&self) -> alloy_primitives::B256 {
        self.hash
    }

    fn from(&self) -> alloy_primitives::Address {
        self.from
    }

    fn to(&self) -> Option<alloy_primitives::Address> {
        self.to
    }

    fn value(&self) -> alloy_primitives::U256 {
        self.value
    }

    fn gas(&self) -> u128 {
        self.gas
    }
}
