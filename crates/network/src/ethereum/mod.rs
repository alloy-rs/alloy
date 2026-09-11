use crate::Network;

mod builder;

mod wallet;
pub use wallet::{EthereumWallet, IntoWallet};

/// Types for a mainnet-like Ethereum network.
#[derive(Clone, Copy, Debug)]
pub struct Ethereum {
    _private: (),
}

impl Network for Ethereum {
    fn try_into_presigned(tx: Self::UnsignedTx) -> Result<Self::TxEnvelope, Self::UnsignedTx> {
        use alloy_primitives::Sealable;
        match tx {
            alloy_consensus::TypedTransaction::Eip8141(tx) => {
                Ok(alloy_consensus::TxEnvelope::Eip8141(tx.seal_slow()))
            }
            tx => Err(tx),
        }
    }

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
