//! Commonly used additional types that are not part of the JSON RPC spec but are often required
//! when working with RPC types, such as [Transaction]

use crate::Transaction;
use alloy_consensus::TxType;
use alloy_eips::eip2718::Eip2718Error;
use alloy_primitives::{TxHash, B256};

/// Additional fields in the context of a block that contains this transaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransactionInfo {
    /// Hash of the transaction.
    pub hash: Option<TxHash>,
    /// Index of the transaction in the block
    pub index: Option<u64>,
    /// Hash of the block.
    pub block_hash: Option<B256>,
    /// Number of the block.
    pub block_number: Option<u64>,
    /// Base fee of the block.
    pub base_fee: Option<u128>,
}

impl TryFrom<Transaction> for TransactionInfo {
    type Error = Eip2718Error;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        Ok(TransactionInfo {
            hash: Some(tx.hash),
            index: tx.transaction_index,
            block_hash: tx.block_hash,
            block_number: tx.block_number,
            base_fee: match tx.transaction_type {
                Some(tx_type) => match tx_type.try_into() {
                    Ok(TxType::Legacy) | Ok(TxType::Eip2930) => tx.gas_price,
                    Ok(TxType::Eip1559) | Ok(TxType::Eip4844) => tx.max_fee_per_gas.map(|fee| {
                        fee.saturating_sub(tx.max_priority_fee_per_gas.unwrap_or_default())
                    }),
                    Err(err) => return Err(err),
                },
                None => None,
            },
        })
    }
}
