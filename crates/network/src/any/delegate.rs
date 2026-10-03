/// Implements every [`Transaction`](alloy_consensus::Transaction) method, including the provided
/// ones, by delegating to an inner transaction.
///
/// - `delegate_transaction!(self => expr)` delegates to the transaction reference `expr`.
/// - `delegate_transaction!(self => match A, B)` delegates to the value of each single-field tuple
///   variant.
macro_rules! delegate_transaction {
    (@call $self:ident [match $($variant:path),+] $method:ident $args:tt) => {
        match $self {
            $($variant(tx) => $crate::any::delegate_transaction!(@call tx [tx] $method $args),)+
        }
    };
    (@call $self:ident [$inner:expr] $method:ident($($arg:ident),*)) => {
        alloy_consensus::Transaction::$method($inner $(, $arg)*)
    };
    (@methods $self:ident $target:tt) => {
        #[inline]
        fn chain_id(&$self) -> Option<alloy_primitives::ChainId> {
            $crate::any::delegate_transaction!(@call $self $target chain_id())
        }

        #[inline]
        fn nonce(&$self) -> u64 {
            $crate::any::delegate_transaction!(@call $self $target nonce())
        }

        #[inline]
        fn gas_limit(&$self) -> u64 {
            $crate::any::delegate_transaction!(@call $self $target gas_limit())
        }

        #[inline]
        fn gas_price(&$self) -> Option<u128> {
            $crate::any::delegate_transaction!(@call $self $target gas_price())
        }

        #[inline]
        fn max_fee_per_gas(&$self) -> u128 {
            $crate::any::delegate_transaction!(@call $self $target max_fee_per_gas())
        }

        #[inline]
        fn max_priority_fee_per_gas(&$self) -> Option<u128> {
            $crate::any::delegate_transaction!(@call $self $target max_priority_fee_per_gas())
        }

        #[inline]
        fn max_fee_per_blob_gas(&$self) -> Option<u128> {
            $crate::any::delegate_transaction!(@call $self $target max_fee_per_blob_gas())
        }

        #[inline]
        fn priority_fee_or_price(&$self) -> u128 {
            $crate::any::delegate_transaction!(@call $self $target priority_fee_or_price())
        }

        #[inline]
        fn effective_gas_price(&$self, base_fee: Option<u64>) -> u128 {
            $crate::any::delegate_transaction!(@call $self $target effective_gas_price(base_fee))
        }

        #[inline]
        fn effective_tip_per_gas(&$self, base_fee: u64) -> Option<u128> {
            $crate::any::delegate_transaction!(@call $self $target effective_tip_per_gas(base_fee))
        }

        #[inline]
        fn is_dynamic_fee(&$self) -> bool {
            $crate::any::delegate_transaction!(@call $self $target is_dynamic_fee())
        }

        #[inline]
        fn kind(&$self) -> alloy_primitives::TxKind {
            $crate::any::delegate_transaction!(@call $self $target kind())
        }

        #[inline]
        fn is_create(&$self) -> bool {
            $crate::any::delegate_transaction!(@call $self $target is_create())
        }

        #[inline]
        fn to(&$self) -> Option<alloy_primitives::Address> {
            $crate::any::delegate_transaction!(@call $self $target to())
        }

        #[inline]
        fn value(&$self) -> alloy_primitives::U256 {
            $crate::any::delegate_transaction!(@call $self $target value())
        }

        #[inline]
        fn input(&$self) -> &alloy_primitives::Bytes {
            $crate::any::delegate_transaction!(@call $self $target input())
        }

        #[inline]
        fn function_selector(&$self) -> Option<&alloy_primitives::Selector> {
            $crate::any::delegate_transaction!(@call $self $target function_selector())
        }

        #[inline]
        fn access_list(&$self) -> Option<&alloy_eips::eip2930::AccessList> {
            $crate::any::delegate_transaction!(@call $self $target access_list())
        }

        #[inline]
        fn blob_versioned_hashes(&$self) -> Option<&[alloy_primitives::B256]> {
            $crate::any::delegate_transaction!(@call $self $target blob_versioned_hashes())
        }

        #[inline]
        fn blob_count(&$self) -> Option<u64> {
            $crate::any::delegate_transaction!(@call $self $target blob_count())
        }

        #[inline]
        fn blob_gas_used(&$self) -> Option<u64> {
            $crate::any::delegate_transaction!(@call $self $target blob_gas_used())
        }

        #[inline]
        fn authorization_list(&$self) -> Option<&[alloy_eips::eip7702::SignedAuthorization]> {
            $crate::any::delegate_transaction!(@call $self $target authorization_list())
        }

        #[inline]
        fn authorization_count(&$self) -> Option<u64> {
            $crate::any::delegate_transaction!(@call $self $target authorization_count())
        }
    };
    ($self:ident => match $($variant:path),+ $(,)?) => {
        $crate::any::delegate_transaction!(@methods $self [match $($variant),+]);
    };
    ($self:ident => $inner:expr) => {
        $crate::any::delegate_transaction!(@methods $self [$inner]);
    };
}

pub(crate) use delegate_transaction;

#[cfg(test)]
mod tests {
    use crate::{
        AnyRpcTransaction, AnyTxEnvelope, AnyTypedTransaction, UnknownTxEnvelope,
        UnknownTypedTransaction,
    };
    use alloy_consensus::{
        transaction::Recovered, Transaction, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
        TxEip7702, TxEnvelope, TxLegacy, TypedTransaction,
    };
    use alloy_primitives::{address, bytes, Address, Signature, B256, U256};
    use alloy_rpc_types_eth::{AccessList, AccessListItem, Authorization, SignedAuthorization};
    use alloy_serde::WithOtherFields;

    #[track_caller]
    fn assert_delegates(wrapper: &impl Transaction, inner: &impl Transaction) {
        assert_eq!(wrapper.chain_id(), inner.chain_id());
        assert_eq!(wrapper.nonce(), inner.nonce());
        assert_eq!(wrapper.gas_limit(), inner.gas_limit());
        assert_eq!(wrapper.gas_price(), inner.gas_price());
        assert_eq!(wrapper.max_fee_per_gas(), inner.max_fee_per_gas());
        assert_eq!(wrapper.max_priority_fee_per_gas(), inner.max_priority_fee_per_gas());
        assert_eq!(wrapper.max_fee_per_blob_gas(), inner.max_fee_per_blob_gas());
        assert_eq!(wrapper.priority_fee_or_price(), inner.priority_fee_or_price());
        assert_eq!(wrapper.effective_gas_price(Some(1)), inner.effective_gas_price(Some(1)));
        assert_eq!(wrapper.effective_tip_per_gas(1), inner.effective_tip_per_gas(1));
        assert_eq!(wrapper.is_dynamic_fee(), inner.is_dynamic_fee());
        assert_eq!(wrapper.kind(), inner.kind());
        assert_eq!(wrapper.is_create(), inner.is_create());
        assert_eq!(wrapper.to(), inner.to());
        assert_eq!(wrapper.value(), inner.value());
        assert_eq!(wrapper.input(), inner.input());
        assert_eq!(wrapper.function_selector(), inner.function_selector());
        assert_eq!(wrapper.access_list(), inner.access_list());
        assert_eq!(wrapper.blob_versioned_hashes(), inner.blob_versioned_hashes());
        assert_eq!(wrapper.blob_count(), inner.blob_count());
        assert_eq!(wrapper.blob_gas_used(), inner.blob_gas_used());
        assert_eq!(wrapper.authorization_list(), inner.authorization_list());
        assert_eq!(wrapper.authorization_count(), inner.authorization_count());

        // `AnyTypedTransaction` and `AnyTxEnvelope` used to compute this instead of delegating.
        assert_eq!(
            wrapper.priority_fee_or_price(),
            wrapper.max_priority_fee_per_gas().or(wrapper.gas_price()).unwrap_or_default()
        );
    }

    fn unknown() -> UnknownTypedTransaction {
        serde_json::from_value(serde_json::json!({
            "type": "0x7e",
            "chainId": "0x1",
            "nonce": "0x2",
            "gas": "0x3",
            "gasPrice": "0x4",
            "maxFeePerGas": "0x5",
            "maxPriorityFeePerGas": "0x6",
            "maxFeePerBlobGas": "0x7",
            "to": "0x000000000000000000000000000000000000000b",
            "value": "0xd",
            "input": "0x0102030405",
            "accessList": [{
                "address": "0x0000000000000000000000000000000000000014",
                "storageKeys": ["0x0000000000000000000000000000000000000000000000000000000000000015"]
            }],
            "blobVersionedHashes": [
                "0x0000000000000000000000000000000000000000000000000000000000000016"
            ],
            "authorizationList": [{
                "chainId": "0x17",
                "address": "0x0000000000000000000000000000000000000018",
                "nonce": "0x19",
                "yParity": "0x1",
                "r": "0x1a",
                "s": "0x1b"
            }]
        }))
        .unwrap()
    }

    fn ethereum() -> [TypedTransaction; 5] {
        let to = address!("0x000000000000000000000000000000000000000b");
        let value = U256::from(13);
        let input = bytes!("0102030405");
        let access_list = AccessList(vec![AccessListItem {
            address: Address::with_last_byte(20),
            storage_keys: vec![B256::with_last_byte(21)],
        }]);
        [
            TxLegacy {
                chain_id: Some(1),
                nonce: 2,
                gas_limit: 3,
                gas_price: 4,
                to: to.into(),
                value,
                input: input.clone(),
            }
            .into(),
            TxEip2930 {
                chain_id: 1,
                nonce: 2,
                gas_limit: 3,
                gas_price: 4,
                to: to.into(),
                value,
                access_list: access_list.clone(),
                input: input.clone(),
            }
            .into(),
            TxEip1559 {
                chain_id: 1,
                nonce: 2,
                gas_limit: 3,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 6,
                to: to.into(),
                value,
                access_list: access_list.clone(),
                input: input.clone(),
            }
            .into(),
            TypedTransaction::Eip4844(TxEip4844Variant::TxEip4844(TxEip4844 {
                chain_id: 1,
                nonce: 2,
                gas_limit: 3,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 6,
                max_fee_per_blob_gas: 7,
                to,
                value,
                access_list: access_list.clone(),
                blob_versioned_hashes: vec![B256::with_last_byte(22)],
                input: input.clone(),
            })),
            TxEip7702 {
                chain_id: 1,
                nonce: 2,
                gas_limit: 3,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 6,
                to,
                value,
                access_list,
                authorization_list: vec![SignedAuthorization::new_unchecked(
                    Authorization { chain_id: U256::from(23), address: to, nonce: 25 },
                    1,
                    U256::from(26),
                    U256::from(27),
                )],
                input,
            }
            .into(),
        ]
    }

    #[test]
    fn any_wrappers_delegate_every_method() {
        for tx in ethereum() {
            assert_delegates(&AnyTypedTransaction::Ethereum(tx.clone()), &tx);

            let envelope: TxEnvelope = tx.into_envelope(Signature::test_signature());
            let any_envelope = AnyTxEnvelope::Ethereum(envelope.clone());
            assert_delegates(&any_envelope, &envelope);

            let rpc_tx = alloy_rpc_types_eth::Transaction {
                inner: Recovered::new_unchecked(any_envelope, Address::with_last_byte(30)),
                block_hash: None,
                block_number: None,
                transaction_index: None,
                effective_gas_price: None,
                block_timestamp: None,
            };
            assert_delegates(
                &AnyRpcTransaction::new(WithOtherFields::new(rpc_tx.clone())),
                &rpc_tx,
            );
        }

        let unknown = unknown();
        assert_delegates(&AnyTypedTransaction::Unknown(unknown.clone()), &unknown);

        let unknown_envelope = UnknownTxEnvelope { hash: B256::ZERO, inner: unknown.clone() };
        assert_delegates(&unknown_envelope, &unknown);
        assert_delegates(&AnyTxEnvelope::Unknown(unknown_envelope.clone()), &unknown_envelope);
    }
}
