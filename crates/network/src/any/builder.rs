use std::ops::{Deref, DerefMut};

use alloy_consensus::BlobTransactionSidecar;
use alloy_rpc_types::{AccessList, TransactionRequest, WithOtherFields};

use crate::{
    any::AnyNetwork, ethereum::build_unsigned, BuilderResult, Network, TransactionBuilder,
};

impl TransactionBuilder<AnyNetwork> for WithOtherFields<TransactionRequest> {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        self.deref().chain_id()
    }

    fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
        self.deref_mut().set_chain_id(chain_id)
    }

    fn nonce(&self) -> Option<u64> {
        self.deref().nonce()
    }

    fn set_nonce(&mut self, nonce: u64) {
        self.deref_mut().set_nonce(nonce)
    }

    fn input(&self) -> Option<&alloy_primitives::Bytes> {
        self.deref().input()
    }

    fn set_input(&mut self, input: alloy_primitives::Bytes) {
        self.deref_mut().set_input(input);
    }

    fn from(&self) -> Option<alloy_primitives::Address> {
        self.deref().from()
    }

    fn set_from(&mut self, from: alloy_primitives::Address) {
        self.deref_mut().set_from(from);
    }

    fn to(&self) -> Option<alloy_primitives::TxKind> {
        self.deref().to()
    }

    fn set_to(&mut self, to: alloy_primitives::TxKind) {
        self.deref_mut().set_to(to)
    }

    fn value(&self) -> Option<alloy_primitives::U256> {
        self.deref().value()
    }

    fn set_value(&mut self, value: alloy_primitives::U256) {
        self.deref_mut().set_value(value)
    }

    fn gas_price(&self) -> Option<u128> {
        self.deref().gas_price()
    }

    fn set_gas_price(&mut self, gas_price: u128) {
        self.deref_mut().set_gas_price(gas_price);
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        self.deref().max_fee_per_gas()
    }

    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: u128) {
        self.deref_mut().set_max_fee_per_gas(max_fee_per_gas);
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.deref().max_priority_fee_per_gas()
    }

    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: u128) {
        self.deref_mut().set_max_priority_fee_per_gas(max_priority_fee_per_gas);
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.deref().max_fee_per_blob_gas()
    }

    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: u128) {
        self.deref_mut().set_max_fee_per_blob_gas(max_fee_per_blob_gas)
    }

    fn gas_limit(&self) -> Option<u128> {
        self.deref().gas_limit()
    }

    fn set_gas_limit(&mut self, gas_limit: u128) {
        self.deref_mut().set_gas_limit(gas_limit);
    }

    /// Get the EIP-2930 access list for the transaction.
    fn access_list(&self) -> Option<&AccessList> {
        self.deref().access_list()
    }

    /// Sets the EIP-2930 access list.
    fn set_access_list(&mut self, access_list: AccessList) {
        self.deref_mut().set_access_list(access_list)
    }

    fn blob_sidecar(&self) -> Option<&BlobTransactionSidecar> {
        self.deref().blob_sidecar()
    }

    fn set_blob_sidecar(&mut self, sidecar: BlobTransactionSidecar) {
        self.deref_mut().set_blob_sidecar(sidecar)
    }

    fn build_unsigned(self) -> BuilderResult<<AnyNetwork as Network>::UnsignedTx> {
        build_unsigned::<AnyNetwork>(self.inner)
    }

    async fn build<S: crate::NetworkSigner<AnyNetwork>>(
        self,
        signer: &S,
    ) -> BuilderResult<alloy_consensus::TxEnvelope> {
        Ok(signer.sign_transaction(self.build_unsigned()?).await?)
    }
}
