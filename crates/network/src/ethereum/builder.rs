use crate::{
    BuildResult, Ethereum, Network, NetworkTransactionBuilder, NetworkWallet, TransactionBuilder,
    TransactionBuilderError,
};
use alloy_consensus::{TxType, TypedTransaction};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, U256};
use alloy_rpc_types_eth::{request::TransactionRequest, AccessList, TransactionInputKind};

impl TransactionBuilder for TransactionRequest {
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

    fn take_nonce(&mut self) -> Option<u64> {
        self.nonce.take()
    }

    fn input(&self) -> Option<&Bytes> {
        self.input.input()
    }

    fn set_input<T: Into<Bytes>>(&mut self, input: T) {
        self.input.input = Some(input.into());
    }

    fn set_input_kind<T: Into<Bytes>>(&mut self, input: T, kind: TransactionInputKind) {
        match kind {
            TransactionInputKind::Input => self.input.input = Some(input.into()),
            TransactionInputKind::Data => self.input.data = Some(input.into()),
            TransactionInputKind::Both => {
                let bytes = input.into();
                self.input.input = Some(bytes.clone());
                self.input.data = Some(bytes);
            }
        }
    }

    fn from(&self) -> Option<Address> {
        self.from
    }

    fn set_from(&mut self, from: Address) {
        self.from = Some(from);
    }

    fn kind(&self) -> Option<TxKind> {
        self.to
    }

    fn clear_kind(&mut self) {
        self.to = None;
    }

    fn set_kind(&mut self, kind: TxKind) {
        self.to = Some(kind);
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

    fn gas_limit(&self) -> Option<u64> {
        self.gas
    }

    fn set_gas_limit(&mut self, gas_limit: u64) {
        self.gas = Some(gas_limit);
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.access_list.as_ref()
    }

    fn set_access_list(&mut self, access_list: AccessList) {
        self.access_list = Some(access_list);
    }
}

impl NetworkTransactionBuilder<Ethereum> for TransactionRequest {
    fn can_submit(&self) -> bool {
        // value and data may be None. If they are, they will be set to default.
        // gas fields and nonce may be None, if they are, they will be populated
        // with default values by the RPC server
        self.from.is_some()
    }

    fn can_build(&self) -> bool {
        self.complete_preferred().is_ok()
    }

    fn complete_type(&self, ty: TxType) -> Result<(), Vec<&'static str>> {
        match ty {
            TxType::Legacy => self.complete_legacy(),
            TxType::Eip2930 => self.complete_2930(),
            TxType::Eip1559 => self.complete_1559(),
            TxType::Eip4844 => self.complete_4844(),
            TxType::Eip7702 => self.complete_7702(),
        }
    }

    #[doc(alias = "output_transaction_type")]
    fn output_tx_type(&self) -> TxType {
        self.preferred_type()
    }

    #[doc(alias = "output_transaction_type_checked")]
    fn output_tx_type_checked(&self) -> Option<TxType> {
        self.buildable_type()
    }

    fn prep_for_submission(&mut self) {
        self.transaction_type = Some(self.preferred_type() as u8);
        self.trim_conflicting_keys();
        self.populate_blob_hashes();
    }

    fn build_unsigned(self) -> BuildResult<TypedTransaction, Ethereum> {
        if let Err((tx_type, missing)) = self.missing_keys() {
            return Err(TransactionBuilderError::InvalidTransactionRequest(tx_type, missing)
                .into_unbuilt(self));
        }
        Ok(self.build_typed_tx().expect("checked by missing_keys"))
    }

    async fn build<W: NetworkWallet<Ethereum>>(
        self,
        wallet: &W,
    ) -> Result<<Ethereum as Network>::TxEnvelope, TransactionBuilderError<Ethereum>> {
        Ok(wallet.sign_request(self).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        NetworkTransactionBuilder, TransactionBuilder, TransactionBuilder4844,
        TransactionBuilder7702, TransactionBuilderError,
    };
    use alloy_consensus::{
        transaction::Recovered, BlobTransactionSidecar, SignableTransaction, TxEip1559, TxEip2930,
        TxEnvelope, TxLegacy, TxType, Typed2718, TypedTransaction,
    };
    use alloy_eips::eip7702::Authorization;
    use alloy_primitives::{Address, Bytes, Signature, TxKind, B256, U160, U256};
    use alloy_rpc_types_eth::{AccessList, TransactionRequest};
    use std::str::FromStr;

    #[test]
    fn from_eip1559_to_tx_req() {
        let tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21_000,
            to: Address::ZERO.into(),
            max_priority_fee_per_gas: 20e9 as u128,
            max_fee_per_gas: 20e9 as u128,
            ..Default::default()
        };
        let tx_req: TransactionRequest = tx.into();
        tx_req.build_unsigned().unwrap();
    }

    #[test]
    fn creation_requests_are_buildable() {
        let requests: [(TxType, TransactionRequest); 3] = [
            (
                TxType::Legacy,
                TxLegacy {
                    chain_id: Some(1),
                    nonce: 7,
                    gas_price: 2,
                    gas_limit: 53_000,
                    to: TxKind::Create,
                    value: U256::from(3),
                    input: Bytes::from_static(&[0x60, 0x00, 0x60, 0x00]),
                }
                .into(),
            ),
            (
                TxType::Eip2930,
                TxEip2930 {
                    chain_id: 1,
                    nonce: 7,
                    gas_price: 2,
                    gas_limit: 53_000,
                    to: TxKind::Create,
                    value: U256::from(3),
                    access_list: Default::default(),
                    input: Bytes::from_static(&[0x60, 0x00, 0x60, 0x00]),
                }
                .into(),
            ),
            (
                TxType::Eip1559,
                TxEip1559 {
                    chain_id: 1,
                    nonce: 7,
                    gas_limit: 53_000,
                    max_fee_per_gas: 2,
                    max_priority_fee_per_gas: 1,
                    to: TxKind::Create,
                    value: U256::from(3),
                    access_list: Default::default(),
                    input: Bytes::from_static(&[0x60, 0x00, 0x60, 0x00]),
                }
                .into(),
            ),
        ];

        for (tx_type, request) in requests {
            assert_eq!(request.to, Some(TxKind::Create));
            assert_eq!(request.output_tx_type(), tx_type);
            assert!(request.can_build());
            assert!(request.complete_preferred().is_ok());
            assert_eq!(request.build_unsigned().unwrap().ty(), tx_type as u8);
        }
    }

    #[test]
    fn can_build_requires_preferred_type_completeness() {
        let requests: [(TxType, TransactionRequest); 5] = [
            (
                TxType::Legacy,
                TransactionRequest {
                    nonce: Some(0),
                    gas: Some(21_000),
                    gas_price: Some(1),
                    ..Default::default()
                },
            ),
            (
                TxType::Eip2930,
                TransactionRequest {
                    nonce: Some(0),
                    gas: Some(21_000),
                    gas_price: Some(1),
                    access_list: Some(Default::default()),
                    ..Default::default()
                },
            ),
            (
                TxType::Eip1559,
                TransactionRequest {
                    nonce: Some(0),
                    gas: Some(21_000),
                    max_fee_per_gas: Some(1),
                    max_priority_fee_per_gas: Some(1),
                    ..Default::default()
                },
            ),
            (
                TxType::Eip4844,
                TransactionRequest {
                    to: Some(TxKind::Call(Address::ZERO)),
                    nonce: Some(0),
                    gas: Some(21_000),
                    max_fee_per_gas: Some(1),
                    max_priority_fee_per_gas: Some(1),
                    blob_versioned_hashes: Some(vec![]),
                    ..Default::default()
                },
            ),
            (
                TxType::Eip7702,
                TransactionRequest {
                    to: Some(TxKind::Create),
                    nonce: Some(0),
                    gas: Some(21_000),
                    max_fee_per_gas: Some(1),
                    max_priority_fee_per_gas: Some(1),
                    authorization_list: Some(vec![]),
                    ..Default::default()
                },
            ),
        ];

        for (tx_type, request) in requests {
            assert_eq!(request.output_tx_type(), tx_type);
            assert!(request.complete_preferred().is_err());
            assert!(!request.can_build());
            assert!(request.build_unsigned().is_err());
        }
    }

    #[test]
    fn test_4844_when_sidecar() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(0)
            .with_max_fee_per_gas(0)
            .with_max_priority_fee_per_gas(0)
            .with_to(Address::ZERO)
            .with_blob_sidecar_4844(BlobTransactionSidecar::default())
            .with_max_fee_per_blob_gas(0);

        let tx = request.clone().build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip4844(_)));

        let tx = request.with_gas_price(0).build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip4844(_)));
    }

    #[test]
    fn test_2930_when_access_list() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(0)
            .with_max_fee_per_gas(0)
            .with_max_priority_fee_per_gas(0)
            .with_to(Address::ZERO)
            .with_gas_price(0)
            .with_access_list(AccessList::default());

        let tx = request.build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip2930(_)));
    }

    #[test]
    fn test_7702_when_authorization_list() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(0)
            .with_max_fee_per_gas(0)
            .with_max_priority_fee_per_gas(0)
            .with_to(Address::ZERO)
            .with_access_list(AccessList::default())
            .with_authorization_list(vec![(Authorization {
                chain_id: U256::from(1),
                address: Address::left_padding_from(&[1]),
                nonce: 1u64,
            })
            .into_signed(Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())],);

        let tx = request.build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip7702(_)));
    }

    #[test]
    fn test_default_to_1559() {
        let request = TransactionRequest::default()
            .with_nonce(1)
            .with_gas_limit(0)
            .with_max_fee_per_gas(0)
            .with_max_priority_fee_per_gas(0)
            .with_to(Address::ZERO);

        let tx = request.clone().build_unsigned().unwrap();

        assert!(matches!(tx, TypedTransaction::Eip1559(_)));

        let request = request.with_gas_price(0);
        let tx = request.build_unsigned().unwrap();
        assert!(matches!(tx, TypedTransaction::Legacy(_)));
    }

    #[test]
    fn test_fail_when_sidecar_and_access_list() {
        let request = TransactionRequest::default()
            .with_blob_sidecar_4844(BlobTransactionSidecar::default())
            .with_access_list(AccessList::default());

        let error = request.build_unsigned().unwrap_err();

        assert!(matches!(error.error, TransactionBuilderError::InvalidTransactionRequest(_, _)));
    }

    #[test]
    fn test_invalid_legacy_fields() {
        let request = TransactionRequest::default().with_gas_price(0);

        let error = request.build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error.error
        else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Legacy);
        assert_eq!(errors.len(), 3);
        assert!(errors.contains(&"to"));
        assert!(errors.contains(&"nonce"));
        assert!(errors.contains(&"gas_limit"));
    }

    #[test]
    fn test_invalid_1559_fields() {
        let request = TransactionRequest::default();

        let error = request.build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error.error
        else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip1559);
        assert_eq!(errors.len(), 5);
        assert!(errors.contains(&"to"));
        assert!(errors.contains(&"nonce"));
        assert!(errors.contains(&"gas_limit"));
        assert!(errors.contains(&"max_priority_fee_per_gas"));
        assert!(errors.contains(&"max_fee_per_gas"));
    }

    #[test]
    fn test_invalid_2930_fields() {
        let request = TransactionRequest::default()
            .with_access_list(AccessList::default())
            .with_gas_price(Default::default());

        let error = request.build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error.error
        else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip2930);
        assert_eq!(errors.len(), 3);
        assert!(errors.contains(&"to"));
        assert!(errors.contains(&"nonce"));
        assert!(errors.contains(&"gas_limit"));
    }

    #[test]
    fn test_invalid_4844_fields() {
        let request =
            TransactionRequest::default().with_blob_sidecar_4844(BlobTransactionSidecar::default());

        let error = request.build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error.error
        else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip4844);
        assert_eq!(errors.len(), 6);
        assert!(errors.contains(&"to"));
        assert!(errors.contains(&"nonce"));
        assert!(errors.contains(&"gas_limit"));
        assert!(errors.contains(&"max_priority_fee_per_gas"));
        assert!(errors.contains(&"max_fee_per_gas"));
        assert!(errors.contains(&"max_fee_per_blob_gas"));
    }

    #[test]
    fn test_tx_response_into_req() {
        let from = Address::from(U160::from(1));
        let to = Address::from(U160::from(1));
        let access_list_item = alloy_rpc_types_eth::AccessListItem {
            address: Address::from(U160::from(3)),
            storage_keys: vec![B256::from(U256::from(4)), B256::from(U256::from(5))],
        };
        let tx = TxEip1559 {
            chain_id: 1337,
            nonce: 12,
            max_priority_fee_per_gas: 123,
            max_fee_per_gas: 1234,
            gas_limit: 21000,
            to: TxKind::Call(to),
            value: U256::from(111),
            access_list: AccessList::from(vec![access_list_item.clone()]),
            input: Bytes::new(),
        };
        let envelope =
            TxEnvelope::Eip1559(tx.into_signed(Signature::new(U256::ZERO, U256::ZERO, false)));
        let tx_response = alloy_rpc_types_eth::Transaction {
            inner: Recovered::new_unchecked(envelope, from),
            effective_gas_price: Some(1000),
            block_hash: None,
            block_number: None,
            block_timestamp: None,
            transaction_index: None,
        };

        // Convert the transaction response into a transaction request via
        // From<TransactionResponse>, and check that the fields are correctly populated.
        let req: TransactionRequest = tx_response.into();

        assert_eq!(TransactionBuilder::from(&req).unwrap(), from);
        assert_eq!(TransactionBuilder::chain_id(&req).unwrap(), 1337);
        assert_eq!(TransactionBuilder::nonce(&req).unwrap(), 12);
        assert_eq!(TransactionBuilder::max_priority_fee_per_gas(&req).unwrap(), 123);
        assert_eq!(TransactionBuilder::max_fee_per_gas(&req).unwrap(), 1234);
        assert_eq!(TransactionBuilder::gas_limit(&req).unwrap(), 21000);
        assert_eq!(TransactionBuilder::to(&req).unwrap(), to);
        assert_eq!(TransactionBuilder::value(&req).unwrap(), 111);
        assert_eq!(
            *TransactionBuilder::access_list(&req).unwrap(),
            AccessList::from(vec![access_list_item])
        );
        assert_eq!(*TransactionBuilder::input(&req).unwrap(), Bytes::new());
    }

    #[test]
    fn test_invalid_7702_fields() {
        let request = TransactionRequest::default().with_authorization_list(vec![]);

        let error = request.build_unsigned().unwrap_err();

        let TransactionBuilderError::InvalidTransactionRequest(tx_type, errors) = error.error
        else {
            panic!("wrong variant")
        };

        assert_eq!(tx_type, TxType::Eip7702);
        assert_eq!(errors.len(), 5);
        assert!(errors.contains(&"to"));
        assert!(errors.contains(&"nonce"));
        assert!(errors.contains(&"gas_limit"));
        assert!(errors.contains(&"max_priority_fee_per_gas"));
        assert!(errors.contains(&"max_fee_per_gas"));
    }
}
