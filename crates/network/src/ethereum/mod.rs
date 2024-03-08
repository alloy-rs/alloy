use crate::Network;

mod builder;

mod signer;
pub use signer::EthereumSigner;

/// Types for a mainnet-like Ethereum network.
#[derive(Debug, Clone, Copy)]
pub struct Ethereum;

impl Network for Ethereum {
    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = alloy_rpc_types::transaction::TransactionRequest;

    type TransactionResponse = alloy_rpc_types::Transaction;

    type ReceiptResponse = alloy_rpc_types::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types::Header;
}
