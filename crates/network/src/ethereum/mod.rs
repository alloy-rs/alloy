use crate::Network;

mod builder;

mod wallet;
pub use wallet::EthereumWallet;

/// Types for a mainnet-like Ethereum network.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Ethereum;

impl Network for Ethereum {
    type TxType = alloy_consensus::TxType;

    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = alloy_rpc_types_eth::transaction::TransactionRequest;

    type TransactionResponse = alloy_rpc_types_eth::Transaction;

    type ReceiptResponse = alloy_rpc_types_eth::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::Header;

    type BlockResponse = alloy_rpc_types_eth::Block;
}
