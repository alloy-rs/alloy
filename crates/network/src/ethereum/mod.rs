use crate::{Block, Header, Network, ReceiptResponse, Transaction};

mod builder;
use alloy_primitives::U256;
pub(crate) use builder::build_unsigned;

mod signer;
pub use signer::EthereumSigner;

/// Types for a mainnet-like Ethereum network.
#[derive(Debug, Clone, Copy)]
pub struct Ethereum {
    _private: (),
}

impl Network for Ethereum {
    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = alloy_rpc_types::transaction::TransactionRequest;

    type TransactionResponse = alloy_rpc_types::Transaction;

    type ReceiptResponse = alloy_rpc_types::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types::Header;

    type BlockResponse = alloy_rpc_types::Block;
}

impl ReceiptResponse for alloy_rpc_types::TransactionReceipt {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }
}

impl Block<Ethereum> for alloy_rpc_types::Block {
    fn header(&self) -> &alloy_rpc_types::Header {
        &self.header
    }

    fn transactions(&self) -> &crate::TransactionList<alloy_rpc_types::Transaction> {
        &self.transactions
    }
}

impl Header for alloy_rpc_types::Header {
    fn base_fee_per_gas(&self) -> Option<U256> {
        self.base_fee_per_gas
    }

    fn next_block_blob_fee(&self) -> Option<u128> {
        self.next_block_blob_fee()
    }
}

impl Transaction for alloy_rpc_types::Transaction {}
