/// Implements every [`Transaction`](crate::Transaction) method, including the provided ones, by
/// delegating to an inner transaction.
///
/// - `delegate_transaction!(self => expr)` delegates to the transaction reference `expr`.
/// - `delegate_transaction!(self => match A, B)` delegates to the value of each single-field tuple
///   variant.
macro_rules! delegate_transaction {
    (@call $self:ident [match $($variant:path),+] $method:ident $args:tt) => {
        match $self {
            $($variant(tx) => $crate::transaction::delegate_transaction!(@call tx [tx] $method $args),)+
        }
    };
    (@call $self:ident [$inner:expr] $method:ident($($arg:ident),*)) => {
        $crate::Transaction::$method($inner $(, $arg)*)
    };
    (@methods $self:ident $target:tt) => {
        #[inline]
        fn chain_id(&$self) -> Option<alloy_primitives::ChainId> {
            $crate::transaction::delegate_transaction!(@call $self $target chain_id())
        }

        #[inline]
        fn nonce(&$self) -> u64 {
            $crate::transaction::delegate_transaction!(@call $self $target nonce())
        }

        #[inline]
        fn gas_limit(&$self) -> u64 {
            $crate::transaction::delegate_transaction!(@call $self $target gas_limit())
        }

        #[inline]
        fn gas_price(&$self) -> Option<u128> {
            $crate::transaction::delegate_transaction!(@call $self $target gas_price())
        }

        #[inline]
        fn max_fee_per_gas(&$self) -> u128 {
            $crate::transaction::delegate_transaction!(@call $self $target max_fee_per_gas())
        }

        #[inline]
        fn max_priority_fee_per_gas(&$self) -> Option<u128> {
            $crate::transaction::delegate_transaction!(
                @call $self $target max_priority_fee_per_gas()
            )
        }

        #[inline]
        fn max_fee_per_blob_gas(&$self) -> Option<u128> {
            $crate::transaction::delegate_transaction!(@call $self $target max_fee_per_blob_gas())
        }

        #[inline]
        fn priority_fee_or_price(&$self) -> u128 {
            $crate::transaction::delegate_transaction!(@call $self $target priority_fee_or_price())
        }

        #[inline]
        fn effective_gas_price(&$self, base_fee: Option<u64>) -> u128 {
            $crate::transaction::delegate_transaction!(
                @call $self $target effective_gas_price(base_fee)
            )
        }

        #[inline]
        fn effective_tip_per_gas(&$self, base_fee: u64) -> Option<u128> {
            $crate::transaction::delegate_transaction!(
                @call $self $target effective_tip_per_gas(base_fee)
            )
        }

        #[inline]
        fn is_dynamic_fee(&$self) -> bool {
            $crate::transaction::delegate_transaction!(@call $self $target is_dynamic_fee())
        }

        #[inline]
        fn kind(&$self) -> alloy_primitives::TxKind {
            $crate::transaction::delegate_transaction!(@call $self $target kind())
        }

        #[inline]
        fn is_create(&$self) -> bool {
            $crate::transaction::delegate_transaction!(@call $self $target is_create())
        }

        #[inline]
        fn to(&$self) -> Option<alloy_primitives::Address> {
            $crate::transaction::delegate_transaction!(@call $self $target to())
        }

        #[inline]
        fn value(&$self) -> alloy_primitives::U256 {
            $crate::transaction::delegate_transaction!(@call $self $target value())
        }

        #[inline]
        fn input(&$self) -> &alloy_primitives::Bytes {
            $crate::transaction::delegate_transaction!(@call $self $target input())
        }

        #[inline]
        fn function_selector(&$self) -> Option<&alloy_primitives::Selector> {
            $crate::transaction::delegate_transaction!(@call $self $target function_selector())
        }

        #[inline]
        fn access_list(&$self) -> Option<&alloy_eips::eip2930::AccessList> {
            $crate::transaction::delegate_transaction!(@call $self $target access_list())
        }

        #[inline]
        fn blob_versioned_hashes(&$self) -> Option<&[alloy_primitives::B256]> {
            $crate::transaction::delegate_transaction!(@call $self $target blob_versioned_hashes())
        }

        #[inline]
        fn blob_count(&$self) -> Option<u64> {
            $crate::transaction::delegate_transaction!(@call $self $target blob_count())
        }

        #[inline]
        fn blob_gas_used(&$self) -> Option<u64> {
            $crate::transaction::delegate_transaction!(@call $self $target blob_gas_used())
        }

        #[inline]
        fn authorization_list(&$self) -> Option<&[alloy_eips::eip7702::SignedAuthorization]> {
            $crate::transaction::delegate_transaction!(@call $self $target authorization_list())
        }

        #[inline]
        fn authorization_count(&$self) -> Option<u64> {
            $crate::transaction::delegate_transaction!(@call $self $target authorization_count())
        }
    };
    ($self:ident => match $($variant:path),+ $(,)?) => {
        $crate::transaction::delegate_transaction!(@methods $self [match $($variant),+]);
    };
    ($self:ident => $inner:expr) => {
        $crate::transaction::delegate_transaction!(@methods $self [$inner]);
    };
}

pub(crate) use delegate_transaction;

#[cfg(test)]
mod tests {
    use crate::{
        transaction::Either, Extended, Signed, Transaction, TxEip4844, TxEip4844Variant,
        TxEip4844WithSidecar, Typed2718,
    };
    use alloc::{vec, vec::Vec};
    use alloy_eips::{
        eip2930::{AccessList, AccessListItem},
        eip7702::{Authorization, SignedAuthorization},
    };
    use alloy_primitives::{
        Address, Bytes, ChainId, Sealed, Selector, Signature, TxKind, B256, U256,
    };

    /// Returns a distinct value from every method, and overrides every provided method with a
    /// value the default implementation would not compute.
    #[derive(Debug)]
    struct MockTx {
        input: Bytes,
        selector: Selector,
        access_list: AccessList,
        blob_versioned_hashes: Vec<B256>,
        authorization_list: Vec<SignedAuthorization>,
    }

    impl MockTx {
        fn new() -> Self {
            Self {
                input: Bytes::from_static(&[1, 2, 3, 4, 5]),
                selector: Selector::new([0xaa; 4]),
                access_list: AccessList(vec![AccessListItem {
                    address: Address::with_last_byte(20),
                    storage_keys: vec![B256::with_last_byte(21)],
                }]),
                blob_versioned_hashes: vec![B256::with_last_byte(22)],
                authorization_list: vec![SignedAuthorization::new_unchecked(
                    Authorization {
                        chain_id: U256::from(23),
                        address: Address::with_last_byte(24),
                        nonce: 25,
                    },
                    1,
                    U256::from(26),
                    U256::from(27),
                )],
            }
        }
    }

    impl Typed2718 for MockTx {
        fn ty(&self) -> u8 {
            0x7f
        }
    }

    impl Transaction for MockTx {
        fn chain_id(&self) -> Option<ChainId> {
            Some(1)
        }

        fn nonce(&self) -> u64 {
            2
        }

        fn gas_limit(&self) -> u64 {
            3
        }

        fn gas_price(&self) -> Option<u128> {
            Some(4)
        }

        fn max_fee_per_gas(&self) -> u128 {
            5
        }

        fn max_priority_fee_per_gas(&self) -> Option<u128> {
            Some(6)
        }

        fn max_fee_per_blob_gas(&self) -> Option<u128> {
            Some(7)
        }

        fn priority_fee_or_price(&self) -> u128 {
            8
        }

        fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
            9 + base_fee.unwrap_or_default() as u128
        }

        fn effective_tip_per_gas(&self, base_fee: u64) -> Option<u128> {
            Some(10 + base_fee as u128)
        }

        fn is_dynamic_fee(&self) -> bool {
            false
        }

        fn kind(&self) -> TxKind {
            TxKind::Call(Address::with_last_byte(11))
        }

        fn is_create(&self) -> bool {
            true
        }

        fn to(&self) -> Option<Address> {
            Some(Address::with_last_byte(12))
        }

        fn value(&self) -> U256 {
            U256::from(13)
        }

        fn input(&self) -> &Bytes {
            &self.input
        }

        fn function_selector(&self) -> Option<&Selector> {
            Some(&self.selector)
        }

        fn access_list(&self) -> Option<&AccessList> {
            Some(&self.access_list)
        }

        fn blob_versioned_hashes(&self) -> Option<&[B256]> {
            Some(&self.blob_versioned_hashes)
        }

        fn blob_count(&self) -> Option<u64> {
            Some(14)
        }

        fn blob_gas_used(&self) -> Option<u64> {
            Some(15)
        }

        fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
            Some(&self.authorization_list)
        }

        fn authorization_count(&self) -> Option<u64> {
            Some(16)
        }
    }

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
        assert_eq!(wrapper.effective_gas_price(Some(100)), inner.effective_gas_price(Some(100)));
        assert_eq!(wrapper.effective_tip_per_gas(100), inner.effective_tip_per_gas(100));
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
    }

    #[test]
    fn wrappers_delegate_every_method() {
        let mock = MockTx::new();
        let signed = Signed::new_unchecked(MockTx::new(), Signature::test_signature(), B256::ZERO);
        assert_delegates(&signed, &mock);
        assert_delegates(&Sealed::new_unchecked(MockTx::new(), B256::ZERO), &mock);
        #[cfg(feature = "serde")]
        assert_delegates(&alloy_serde::WithOtherFields::new(MockTx::new()), &mock);
        assert_delegates(&Either::<MockTx, MockTx>::Left(MockTx::new()), &mock);
        assert_delegates(&Either::<MockTx, MockTx>::Right(MockTx::new()), &mock);
        assert_delegates(&Extended::<MockTx, MockTx>::BuiltIn(MockTx::new()), &mock);
        assert_delegates(&Extended::<MockTx, MockTx>::Other(MockTx::new()), &mock);

        let tx = TxEip4844 {
            chain_id: 1,
            nonce: 2,
            gas_limit: 3,
            max_fee_per_gas: 5,
            max_priority_fee_per_gas: 6,
            to: Address::with_last_byte(11),
            value: U256::from(13),
            access_list: mock.access_list.clone(),
            blob_versioned_hashes: mock.blob_versioned_hashes.clone(),
            max_fee_per_blob_gas: 7,
            input: mock.input,
        };
        let with_sidecar = TxEip4844WithSidecar::from_tx_and_sidecar(tx.clone(), ());
        assert_delegates(&with_sidecar, &tx);
        assert_delegates(&TxEip4844Variant::<()>::TxEip4844(tx.clone()), &tx);
        assert_delegates(&TxEip4844Variant::TxEip4844WithSidecar(with_sidecar), &tx);
    }
}
