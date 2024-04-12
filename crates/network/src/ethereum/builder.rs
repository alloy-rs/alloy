use crate::{
    BuilderResult, Ethereum, Network, NetworkSigner, TransactionBuilder, TransactionBuilderError,
};
use alloy_consensus::{
    BlobTransactionSidecar, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxLegacy,
    TypedTransaction,
};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, U256};
use alloy_rpc_types::{request::TransactionRequest, AccessList};

impl TransactionBuilder<Ethereum> for TransactionRequest {
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn set_chain_id(&mut self, chain_id: ChainId) {
        self.chain_id = Some(chain_id);
    }

    fn nonce(&self) -> Option<u64> {
        self.nonce
    }

    fn set_nonce(&mut self, nonce: u64) {
        self.nonce = Some(nonce);
    }

    fn input(&self) -> Option<&Bytes> {
        self.input.input()
    }

    fn set_input(&mut self, input: Bytes) {
        self.input.input = Some(input);
    }

    fn from(&self) -> Option<Address> {
        self.from
    }

    fn set_from(&mut self, from: Address) {
        self.from = Some(from);
    }

    fn to(&self) -> Option<TxKind> {
        self.to.map(TxKind::Call).or(Some(TxKind::Create))
    }

    fn set_to(&mut self, to: TxKind) {
        match to {
            TxKind::Create => self.to = None,
            TxKind::Call(to) => self.to = Some(to),
        }
    }

    fn value(&self) -> Option<U256> {
        self.value
    }

    fn set_value(&mut self, value: U256) {
        self.value = Some(value)
    }

    fn gas_price(&self) -> Option<u128> {
        self.gas_price
    }

    fn set_gas_price(&mut self, gas_price: u128) {
        self.gas_price = Some(gas_price);
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        self.max_fee_per_gas
    }

    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: u128) {
        self.max_fee_per_gas = Some(max_fee_per_gas);
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: u128) {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.max_fee_per_blob_gas
    }

    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: u128) {
        self.max_fee_per_blob_gas = Some(max_fee_per_blob_gas)
    }

    fn gas_limit(&self) -> Option<u128> {
        self.gas
    }

    fn set_gas_limit(&mut self, gas_limit: u128) {
        self.gas = Some(gas_limit);
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.access_list.as_ref()
    }

    fn set_access_list(&mut self, access_list: AccessList) {
        self.access_list = Some(access_list);
    }

    fn blob_sidecar(&self) -> Option<&BlobTransactionSidecar> {
        self.sidecar.as_ref()
    }

    fn set_blob_sidecar(&mut self, sidecar: BlobTransactionSidecar) {
        self.blob_versioned_hashes = Some(sidecar.versioned_hashes().collect());
        self.sidecar = Some(sidecar);
    }

    fn can_submit(&self) -> bool {
        // value and data may be None. If they are, they will be set to default.
        // gas fields and nonce may be None, if they are, they will be populated
        // with default values by the RPC server
        self.from.is_some()
    }

    fn can_build(&self) -> bool {
        // value and data may be none. If they are, they will be set to default
        // values.

        // chain_id and from may be none.
        let common = self.gas.is_some() && self.nonce.is_some();

        let legacy = self.gas_price.is_some();
        let eip2930 = legacy && self.access_list().is_some();

        let eip1559 = self.max_fee_per_gas.is_some() && self.max_priority_fee_per_gas.is_some();

        let eip4844 = eip1559 && self.sidecar.is_some() && self.to.is_some();
        common && (legacy || eip2930 || eip1559 || eip4844)
    }

    fn build_unsigned(self) -> BuilderResult<TypedTransaction> {
        build_unsigned::<Ethereum>(self)
    }

    async fn build<S: NetworkSigner<Ethereum>>(
        self,
        signer: &S,
    ) -> BuilderResult<<Ethereum as Network>::TxEnvelope> {
        Ok(signer.sign_request(self).await?)
    }
}

/// Build an unsigned transaction
pub(crate) fn build_unsigned<N>(request: TransactionRequest) -> BuilderResult<N::UnsignedTx>
where
    N: Network,
    N::UnsignedTx: From<TxLegacy> + From<TxEip1559> + From<TxEip2930> + From<TxEip4844Variant>,
{
    match (
        request.gas_price.as_ref(),
        request.max_fee_per_gas.as_ref(),
        request.access_list.as_ref(),
        request.max_fee_per_blob_gas.as_ref(),
        request.blob_versioned_hashes.as_ref(),
        request.sidecar.as_ref(),
    ) {
        // Legacy transaction
        (Some(_), None, None, None, None, None) => build_legacy(request).map(Into::into),
        // EIP-2930
        // If only accesslist is set, and there are no EIP-1559 fees
        (_, None, Some(_), None, None, None) => build_2930(request).map(Into::into),
        // EIP-1559
        // If EIP-4844 fields are missing
        (None, _, _, None, None, None) => build_1559(request).map(Into::into),
        // EIP-4844
        // All blob fields required
        (None, _, _, Some(_), Some(_), Some(_)) => {
            build_4844(request).map(TxEip4844Variant::from).map(Into::into)
        }
        _ => build_legacy(request).map(Into::into),
    }
}

/// Build a legacy transaction.
fn build_legacy(request: TransactionRequest) -> Result<TxLegacy, TransactionBuilderError> {
    Ok(TxLegacy {
        chain_id: request.chain_id,
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_price: request
            .gas_price
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_price"))?,
        gas_limit: request.gas.ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?,
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
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_priority_fee_per_gas"))?,
        max_fee_per_gas: request
            .max_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_gas"))?,
        gas_limit: request.gas.ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?,
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
        access_list: request.access_list.unwrap_or_default(),
    })
}

/// Build an EIP-2930 transaction.
fn build_2930(request: TransactionRequest) -> Result<TxEip2930, TransactionBuilderError> {
    Ok(TxEip2930 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_price: request
            .gas_price
            .ok_or_else(|| TransactionBuilderError::MissingKey("gas_price"))?,
        gas_limit: request.gas.ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?,
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
        access_list: request.access_list.unwrap_or_default(),
    })
}

/// Build an EIP-4844 transaction.
fn build_4844(request: TransactionRequest) -> Result<TxEip4844, TransactionBuilderError> {
    Ok(TxEip4844 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.ok_or_else(|| TransactionBuilderError::MissingKey("nonce"))?,
        gas_limit: request.gas.ok_or_else(|| TransactionBuilderError::MissingKey("gas_limit"))?,
        max_fee_per_gas: request
            .max_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_gas"))?,
        max_priority_fee_per_gas: request
            .max_priority_fee_per_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_priority_fee_per_gas"))?,
        to: request.to.ok_or_else(|| TransactionBuilderError::MissingKey("to"))?,
        value: request.value.unwrap_or_default(),
        access_list: request.access_list.unwrap_or_default(),
        blob_versioned_hashes: request
            .blob_versioned_hashes
            .ok_or_else(|| TransactionBuilderError::MissingKey("blob_versioned_hashes"))?,
        max_fee_per_blob_gas: request
            .max_fee_per_blob_gas
            .ok_or_else(|| TransactionBuilderError::MissingKey("max_fee_per_blob_gas"))?,
        input: request.input.into_input().unwrap_or_default(),
    })
}
