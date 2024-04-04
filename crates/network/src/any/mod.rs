use crate::{BlockResponse, HeaderResponse, Network, ReceiptResponse};
use alloy_consensus::AnyReceiptEnvelope;
use alloy_rpc_types::{
    Block, Header, Log, Transaction, TransactionList, TransactionReceipt, TransactionRequest,
    WithOtherFields,
};

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

    type ReceiptEnvelope = alloy_consensus::AnyReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = WithOtherFields<TransactionRequest>;

    type TransactionResponse = WithOtherFields<Transaction>;

    type ReceiptResponse = WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>;

    type HeaderResponse = WithOtherFields<Header>;

    type BlockResponse = WithOtherFields<Block<Self::HeaderResponse, Self::TransactionResponse>>;
}

impl ReceiptResponse for WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>> {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }
}

impl BlockResponse<AnyNetwork>
    for WithOtherFields<Block<WithOtherFields<Header>, WithOtherFields<Transaction>>>
{
    fn header(&self) -> &WithOtherFields<Header> {
        &self.header
    }

    fn transactions(&self) -> &TransactionList<WithOtherFields<Transaction>> {
        &self.transactions
    }
}

impl HeaderResponse for WithOtherFields<Header> {
    fn base_fee_per_gas(&self) -> Option<alloy_primitives::U256> {
        todo!()
    }

    fn next_block_blob_fee(&self) -> Option<u128> {
        todo!()
    }
}
