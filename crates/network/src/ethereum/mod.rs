use crate::{BuilderResult, Network, NetworkSigner, TransactionBuilder};

mod receipt;
mod signer;
use alloy_primitives::{Address, TxKind, U256, U64};
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

impl TransactionBuilder<Ethereum> for alloy_rpc_types::TransactionRequest {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        self.chain_id
    }

    fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
        self.chain_id = Some(chain_id);
    }

    fn nonce(&self) -> Option<U64> {
        self.nonce
    }

    fn set_nonce(&mut self, nonce: U64) {
        self.nonce = Some(nonce);
    }

    fn input(&self) -> Option<&alloy_primitives::Bytes> {
        self.input.input()
    }

    fn set_input(&mut self, input: alloy_primitives::Bytes) {
        self.input.input = Some(input);
    }

    fn to(&self) -> Option<alloy_primitives::TxKind> {
        self.to.map(TxKind::Call).or(Some(TxKind::Create))
    }

    fn from(&self) -> Option<Address> {
        self.from
    }

    fn set_from(&mut self, from: Address) {
        self.from = Some(from);
    }

    fn set_to(&mut self, to: alloy_primitives::TxKind) {
        match to {
            TxKind::Create => self.to = None,
            TxKind::Call(to) => self.to = Some(to),
        }
    }

    fn value(&self) -> Option<alloy_primitives::U256> {
        self.value
    }

    fn set_value(&mut self, value: alloy_primitives::U256) {
        self.value = Some(value)
    }

    fn gas_price(&self) -> Option<U256> {
        self.gas_price
    }

    fn set_gas_price(&mut self, gas_price: U256) {
        self.gas_price = Some(gas_price);
    }

    fn max_fee_per_gas(&self) -> Option<U256> {
        self.max_fee_per_gas
    }

    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: U256) {
        self.max_fee_per_gas = Some(max_fee_per_gas);
    }

    fn max_priority_fee_per_gas(&self) -> Option<U256> {
        self.max_priority_fee_per_gas
    }

    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: U256) {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
    }

    fn max_fee_per_blob_gas(&self) -> Option<U256> {
        self.max_fee_per_blob_gas
    }

    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: U256) {
        self.max_fee_per_blob_gas = Some(max_fee_per_blob_gas)
    }

    fn gas_limit(&self) -> Option<U256> {
        self.gas
    }

    fn set_gas_limit(&mut self, gas_limit: U256) {
        self.gas = Some(gas_limit);
    }

    fn build_unsigned(self) -> BuilderResult<<Ethereum as Network>::UnsignedTx> {
        todo!()
    }

    fn build<S: NetworkSigner<Ethereum>>(
        self,
        signer: &S,
    ) -> BuilderResult<<Ethereum as Network>::TxEnvelope> {
        todo!()
    }
}
