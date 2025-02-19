use super::Block;
use crate::block::BlockBody;
use alloy_eips::eip4895::Withdrawals;
/// A trait for ethereum like blocks.
pub trait EthBlock {
    /// Returns reference to withdrawals in the block if present
    fn withdrawals(&self) -> Option<&Withdrawals>;
}

impl<T, H> EthBlock for Block<T, H> {
    fn withdrawals(&self) -> Option<&Withdrawals> {
        self.body.withdrawals.as_ref()
    }
}

/// A trait for Ethereum block body utilities.
pub trait EthBlockBody {
    /// Returns whether or not the block body contains any EIP-4844 transactions.
    fn has_eip4844_transactions(&self) -> bool;
    /// Returns whether or not the block body contains any EIP-7702 transactions.
    fn has_eip7702_transactions(&self) -> bool;
}

impl<T: alloy_eips::Typed2718, H> EthBlockBody for BlockBody<T, H> {
    fn has_eip4844_transactions(&self) -> bool {
        self.transactions.iter().any(|tx| tx.is_eip4844())
    }

    fn has_eip7702_transactions(&self) -> bool {
        self.transactions.iter().any(|tx| tx.is_eip7702())
    }
}
