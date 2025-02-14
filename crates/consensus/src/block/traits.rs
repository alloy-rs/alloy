use super::Block;
use alloy_eips::eip4895::Withdrawals;
/// Minimal block type.
pub trait BlockT {
    /// Returns reference to withdrawals in the block if present
    fn withdrawals(&self) -> Option<&Withdrawals>;
}

impl<T, H> BlockT for Block<T, H> {
    fn withdrawals(&self) -> Option<&Withdrawals> {
        self.body.withdrawals.as_ref()
    }
}
