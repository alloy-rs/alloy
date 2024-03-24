use alloy_rpc_types::{Extra, Header, Transaction, TransactionReceipt, TransactionRequest};

use crate::{Network, ReceiptResponse};

mod builder;

/// Types for a catch-all network.
#[derive(Debug, Clone, Copy)]
pub struct AnyNetwork {
    _private: (),
}

impl Network for AnyNetwork {
    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = Extra<TransactionRequest>;

    type TransactionResponse = Extra<Transaction>;

    type ReceiptResponse = Extra<TransactionReceipt>;

    type HeaderResponse = Extra<Header>;
}

impl ReceiptResponse for Extra<TransactionReceipt> {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }
}
