use crate::{
    transaction::{InvalidTransactionRequestError, InvalidTransactionRequestErrors},
    BuilderResult, Ethereum, Network, NetworkSigner, TransactionBuilder, TransactionBuilderError,
};
use alloy_consensus::{
    BlobTransactionSidecar, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxLegacy, TxType,
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
    if will_build_4844(&request)? {
        build_4844(request).map(TxEip4844Variant::from).map(Into::into)
    } else if will_build_2930(&request)? {
        build_2930(request).map(Into::into)
    } else if will_build_legacy(&request)? {
        build_legacy(request).map(Into::into)
    } else {
        build_1559(request).map(Into::into)
    }
}

fn get_invalid_common_fields(request: &TransactionRequest) -> InvalidTransactionRequestErrors {
    let mut errors = vec![];

    if request.nonce.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("nonce"));
    }

    if request.gas.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("gas_limit"));
    }

    InvalidTransactionRequestErrors(errors)
}

fn get_invalid_1559_fields(request: &TransactionRequest) -> Vec<InvalidTransactionRequestError> {
    let mut errors = vec![];

    if request.max_priority_fee_per_gas.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("max_priority_fee_per_gas"));
    }

    if request.max_fee_per_gas.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("max_fee_per_gas"));
    }

    errors
}

fn will_build_legacy(request: &TransactionRequest) -> Result<bool, TransactionBuilderError> {
    if request.gas_price.is_none() {
        return Ok(false);
    }

    let errors = get_invalid_common_fields(request);

    if errors.is_empty() {
        Ok(true)
    } else {
        Err(TransactionBuilderError::InvalidTransactionRequest(TxType::Legacy, errors))
    }
}

fn will_build_2930(request: &TransactionRequest) -> Result<bool, TransactionBuilderError> {
    if request.access_list.is_none() {
        return Ok(false);
    }

    let mut errors = get_invalid_common_fields(request);

    if request.gas_price.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("gas_price"));
    }

    if request.sidecar.is_some() {
        errors.push(InvalidTransactionRequestError::MutuallyExclusive("access_list", "sidecar"));
    }

    if errors.is_empty() {
        Ok(true)
    } else {
        Err(TransactionBuilderError::InvalidTransactionRequest(TxType::Eip2930, errors))
    }
}

fn will_build_4844(request: &TransactionRequest) -> Result<bool, TransactionBuilderError> {
    if request.sidecar.is_none()
        && request.blob_versioned_hashes.is_none()
        && request.max_fee_per_blob_gas.is_none()
    {
        return Ok(false);
    }

    let mut errors = get_invalid_common_fields(request);

    errors.append(&mut get_invalid_1559_fields(request));

    if request.access_list.is_some() {
        errors.push(InvalidTransactionRequestError::MutuallyExclusive("sidecar", "access_list"))
    }

    if request.sidecar.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("sidecar"));
    }

    if request.blob_versioned_hashes.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("blob_versioned_hashes"));
    }

    if request.blob_versioned_hashes.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("max_fee_per_blob_gas"));
    }

    if request.to.is_none() {
        errors.push(InvalidTransactionRequestError::MissingKey("to"));
    }

    if errors.is_empty() {
        Ok(true)
    } else {
        Err(TransactionBuilderError::InvalidTransactionRequest(TxType::Eip4844, errors))
    }
}

/// Build a legacy transaction.
fn build_legacy(request: TransactionRequest) -> Result<TxLegacy, TransactionBuilderError> {
    Ok(TxLegacy {
        chain_id: request.chain_id,
        nonce: request.nonce.expect("checked in will_build_legacy"),
        gas_price: request.gas_price.expect("checked in will_build_legacy"),
        gas_limit: request.gas.expect("checked in will_build_legacy"),
        to: request.to.into(),
        value: request.value.unwrap_or_default(),
        input: request.input.into_input().unwrap_or_default(),
    })
}

/// Build an EIP-1559 transaction.
fn build_1559(request: TransactionRequest) -> Result<TxEip1559, TransactionBuilderError> {
    let mut errors = get_invalid_common_fields(&request);

    errors.append(&mut get_invalid_1559_fields(&request));

    if errors.is_empty() {
        Ok(TxEip1559 {
            chain_id: request.chain_id.unwrap_or(1),
            nonce: request.nonce.expect("checked in invalid_common_fields"),
            max_priority_fee_per_gas: request
                .max_priority_fee_per_gas
                .expect("checked in invalid_1559_fields"),
            max_fee_per_gas: request.max_fee_per_gas.expect("checked in invalid_1559_fields"),
            gas_limit: request.gas.expect("checked in invalid_common_fields"),
            to: request.to.into(),
            value: request.value.unwrap_or_default(),
            input: request.input.into_input().unwrap_or_default(),
            access_list: request.access_list.unwrap_or_default(),
        })
    } else {
        Err(TransactionBuilderError::InvalidTransactionRequest(TxType::Eip1559, errors))
    }
}

/// Build an EIP-2930 transaction.
fn build_2930(request: TransactionRequest) -> Result<TxEip2930, TransactionBuilderError> {
    Ok(TxEip2930 {
        chain_id: request.chain_id.unwrap_or(1),
        nonce: request.nonce.expect("checked in will_build_2930"),
        gas_price: request.gas_price.expect("checked in will_build_2930"),
        gas_limit: request.gas.expect("checked in will_build_2930"),
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
        nonce: request.nonce.expect("checked in will_build_4844"),
        gas_limit: request.gas.expect("checked in will_build_4844"),
        max_fee_per_gas: request.max_fee_per_gas.expect("checked in will_build_4844"),
        max_priority_fee_per_gas: request
            .max_priority_fee_per_gas
            .expect("checked in will_build_4844"),
        to: request.to.expect("checked in will_build_4844"),
        value: request.value.unwrap_or_default(),
        access_list: request.access_list.unwrap_or_default(),
        blob_versioned_hashes: request.blob_versioned_hashes.expect("checked in will_build_4844"),
        max_fee_per_blob_gas: request.max_fee_per_blob_gas.expect("checked in will_build_4844"),
        input: request.input.into_input().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use alloy_consensus::{BlobTransactionSidecar, TxType, TypedTransaction};
    use alloy_primitives::{Address, U256};
    use alloy_rpc_types::{AccessList, TransactionRequest};

    use crate::{InvalidTransactionRequestError, TransactionBuilder, TransactionBuilderError};

    #[test]
    fn test_4844_when_sidecar() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(U256::default())
            .with_max_fee_per_gas(U256::default())
            .with_max_priority_fee_per_gas(U256::default())
            .with_to(Address::ZERO.into())
            .with_blob_sidecar(BlobTransactionSidecar::default())
            .with_max_fee_per_blob_gas(U256::default());

        let tx = request.clone().build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip4844(_)));

        let tx = request.with_gas_price(U256::default()).build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip4844(_)));
    }

    #[test]
    fn test_2930_when_acces_list() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(U256::default())
            .with_max_fee_per_gas(U256::default())
            .with_max_priority_fee_per_gas(U256::default())
            .with_to(Address::ZERO.into())
            .with_gas_price(U256::default())
            .access_list(AccessList::default());

        let tx = request.build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip2930(_)));
    }

    #[test]
    fn test_default_to_1559() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(U256::default())
            .with_max_fee_per_gas(U256::default())
            .with_max_priority_fee_per_gas(U256::default())
            .with_to(Address::ZERO.into());

        let tx = request.clone().build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip1559(_)));

        let tx = request.with_gas_price(U256::default()).build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Legacy(_)));
    }

    #[test]
    fn test_fail_when_sidecar_and_access_list() {
        let request = TransactionRequest::default()
            .with_blob_sidecar(BlobTransactionSidecar::default())
            .access_list(AccessList::default());

        let error = request.clone().build_unsigned().unwrap_err();

        assert!(matches!(error, TransactionBuilderError::InvalidTransactionRequest(_, _)));
    }

    #[test]
    fn test_invalid_legacy_fields() {
        let request = TransactionRequest::default().with_gas_price(U256::default());

        let error = request.clone().build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Legacy);
        assert_eq!(errors.len(), 2);
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("nonce")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("gas_limit")));
    }

    #[test]
    fn test_invalid_1559_fields() {
        let request = TransactionRequest::default();

        let error = request.clone().build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip1559);
        assert_eq!(errors.len(), 4);
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("nonce")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("gas_limit")));
        assert!(errors
            .contains(&InvalidTransactionRequestError::MissingKey("max_priority_fee_per_gas")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("max_fee_per_gas")));
    }

    #[test]
    fn test_invalid_2930_fields() {
        let request = TransactionRequest::default().access_list(AccessList::default());

        let error = request.clone().build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip2930);
        assert_eq!(errors.len(), 3);
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("nonce")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("gas_limit")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("gas_price")));
    }

    #[test]
    fn test_invalid_4844_fields() {
        let request =
            TransactionRequest::default().with_blob_sidecar(BlobTransactionSidecar::default());

        let error = request.clone().build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip4844);
        assert_eq!(errors.len(), 5);
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("nonce")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("gas_limit")));
        assert!(errors
            .contains(&InvalidTransactionRequestError::MissingKey("max_priority_fee_per_gas")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("max_fee_per_gas")));
        assert!(errors.contains(&InvalidTransactionRequestError::MissingKey("to")));
    }
}
