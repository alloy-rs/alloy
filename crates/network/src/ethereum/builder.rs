use crate::{
    BuilderResult, Ethereum, Network, NetworkSigner, TransactionBuilder, TransactionBuilderError,
};
use alloy_consensus::{TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxLegacy};
use alloy_primitives::{Address, TxKind, U256};
use alloy_rpc_types::request::TransactionRequest;

impl TransactionBuilder<Ethereum> for alloy_rpc_types::TransactionRequest {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        self.chain_id
    }

    fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
        self.chain_id = Some(chain_id);
    }

    fn nonce(&self) -> Option<u64> {
        self.nonce
    }

    fn set_nonce(&mut self, nonce: u64) {
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
        match (
            self.gas_price.as_ref(),
            self.max_fee_per_gas.as_ref(),
            self.access_list.as_ref(),
            self.max_fee_per_blob_gas.as_ref(),
            self.blob_versioned_hashes.as_ref(),
            self.sidecar.as_ref(),
        ) {
            // Legacy transaction
            (Some(_), None, None, None, None, None) => build_legacy(self).map(Into::into),
            // EIP-2930
            // If only accesslist is set, and there are no EIP-1559 fees
            (_, None, Some(_), None, None, None) => build_2930(self).map(Into::into),
            // EIP-1559
            // If EIP-4844 fields are missing
            (None, _, _, None, None, None) => build_1559(self).map(Into::into),
            // EIP-4844
            // All blob fields required
            (None, _, _, Some(_), Some(_), Some(_)) => {
                build_4844(self).map(TxEip4844Variant::from).map(Into::into)
            }
            _ => build_legacy(self).map(Into::into),
        }
    }

    async fn build<S: NetworkSigner<Ethereum>>(
        self,
        signer: &S,
    ) -> BuilderResult<<Ethereum as Network>::TxEnvelope> {
        Ok(signer.sign_transaction(self.build_unsigned()?).await?)
    }
}

/// Build a legacy transaction.
fn build_legacy(request: TransactionRequest) -> Result<TxLegacy, TransactionBuilderError> {
    Ok(TxLegacy {
        chain_id: request.chain_id,
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_price: request
            .gas_price
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_price"))?
            .to(),
        gas_limit: request
            .gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?
            .to(),
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
    })
}

/// Build an EIP-1559 transaction.
fn build_1559(request: TransactionRequest) -> Result<TxEip1559, TransactionBuilderError> {
    Ok(TxEip1559 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        max_priority_fee_per_gas: request
            .max_priority_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_priority_fee_per_gas"))?
            .to(),
        max_fee_per_gas: request
            .max_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_gas"))?
            .to(),
        gas_limit: request
            .gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?
            .to(),
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
        access_list: convert_access_list(request.access_list.unwrap_or_default()),
    })
}

/// Build an EIP-2930 transaction.
fn build_2930(request: TransactionRequest) -> Result<TxEip2930, TransactionBuilderError> {
    Ok(TxEip2930 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_price: request
            .gas_price
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_price"))?
            .to(),
        gas_limit: request
            .gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?
            .to(),
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
        access_list: convert_access_list(request.access_list.unwrap_or_default()),
    })
}

/// Build an EIP-4844 transaction.
fn build_4844(request: TransactionRequest) -> Result<TxEip4844, TransactionBuilderError> {
    Ok(TxEip4844 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_limit: request
            .gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?
            .to(),
        max_fee_per_gas: request
            .max_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_gas"))?
            .to(),
        max_priority_fee_per_gas: request
            .max_priority_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_priority_fee_per_gas"))?
            .to(),
        to: request.to.ok_or_else(|| TransactionBuilderError::MissingKey("to"))?,
        value: request.value.unwrap_or_default(),
        access_list: convert_access_list(request.access_list.unwrap_or_default()),
        blob_versioned_hashes: request
            .blob_versioned_hashes
            .ok_or_else(|| TransactionBuilderError::MissingKey("blob_versioned_hashes"))?,
        max_fee_per_blob_gas: request
            .max_fee_per_blob_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_blob_gas"))?
            .to(),
        input: request.input.into_input().unwrap_or_default(),
    })
}

// todo: these types are almost 1:1, minus rlp decoding and ser/de, should dedupe
fn convert_access_list(list: alloy_rpc_types::AccessList) -> alloy_eips::eip2930::AccessList {
    alloy_eips::eip2930::AccessList(
        list.0
            .into_iter()
            .map(|item| alloy_eips::eip2930::AccessListItem {
                address: item.address,
                storage_keys: item.storage_keys,
            })
            .collect(),
    )
}
