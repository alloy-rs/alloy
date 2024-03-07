use crate::Receipt;
use alloy_consensus::ReceiptWithBloom;
use alloy_primitives::{Bloom, Log};

impl Receipt for alloy_consensus::Receipt {
    fn success(&self) -> bool {
        self.success
    }

    fn bloom(&self) -> Bloom {
        self.bloom_slow()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.cumulative_gas_used
    }

    fn logs(&self) -> &[Log] {
        &self.logs
    }
}

impl Receipt for ReceiptWithBloom {
    fn success(&self) -> bool {
        self.receipt.success
    }

    fn bloom(&self) -> Bloom {
        self.bloom
    }

    fn bloom_cheap(&self) -> Option<Bloom> {
        Some(self.bloom)
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.receipt.cumulative_gas_used
    }

    fn logs(&self) -> &[Log] {
        &self.receipt.logs
    }
}
