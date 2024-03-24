use alloy_rpc_types::{
    other::WithOtherFields, Header, Transaction, TransactionReceipt, TransactionRequest,
};

use crate::{Network, ReceiptResponse};

mod builder;

/// Types for a catch-all network.
///
/// Essentially just returns the regular Ethereum types + a catch all field.
/// This [`Network`] should be used only when the network is not known at
/// compile time.
#[derive(Debug, Clone, Copy)]
pub struct AnyNetwork {
    _private: (),
}

impl Network for AnyNetwork {
    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = WithOtherFields<TransactionRequest>;

    type TransactionResponse = WithOtherFields<Transaction>;

    type ReceiptResponse = WithOtherFields<TransactionReceipt>;

    type HeaderResponse = WithOtherFields<Header>;
}

impl ReceiptResponse for WithOtherFields<TransactionReceipt> {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }
}
