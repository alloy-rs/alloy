use alloy_primitives::{Bytes, ChainId, Signature, B256, U256};
use alloy_rlp::{BufMut, Encodable};

mod common;
pub use common::TxKind;

mod signed;
pub use signed::Signed;

/// Represents a minimal EVM transaction.
pub trait Transaction: std::any::Any + Encodable + Send + Sync + 'static {
    /// The signature type for this transaction.
    ///
    /// This is usually [`alloy_primitives::Signature`], however, it may be different for future
    /// EIP-2718 transaction types, or in other networks. For example, in Optimism, the deposit
    /// transaction signature is the unit type `()`.
    type Signature;

    /// Convert to a signed transaction by adding a signature and computing the
    /// hash.
    fn into_signed(self, signature: Signature) -> Signed<Self, Self::Signature>
    where
        Self: Sized;

    /// Encode with a signature. This encoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    fn encode_signed(&self, signature: &Signature, out: &mut dyn BufMut);

    /// Decode a signed transaction. This decoding is usually RLP, but may be
    /// different for future EIP-2718 transaction types.
    ///
    /// This MUST be the inverse of [`Transaction::encode_signed`].
    fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>>
    where
        Self: Sized;

    /// Calculate the signing hash for the transaction.
    fn signature_hash(&self) -> B256;

    /// Get `data`.
    fn input(&self) -> &[u8];
    /// Get `data`.
    fn input_mut(&mut self) -> &mut Bytes;
    /// Set `data`.
    fn set_input(&mut self, data: Bytes);

    /// Get `to`.
    fn to(&self) -> TxKind;
    /// Set `to`.
    fn set_to(&mut self, to: TxKind);

    /// Get `value`.
    fn value(&self) -> U256;
    /// Set `value`.
    fn set_value(&mut self, value: U256);

    /// Get `chain_id`.
    fn chain_id(&self) -> Option<ChainId>;
    /// Set `chain_id`.
    fn set_chain_id(&mut self, chain_id: ChainId);

    /// Get `nonce`.
    fn nonce(&self) -> u64;
    /// Set `nonce`.
    fn set_nonce(&mut self, nonce: u64);

    /// Get `gas_limit`.
    fn gas_limit(&self) -> u64;
    /// Set `gas_limit`.
    fn set_gas_limit(&mut self, limit: u64);

    /// Get `gas_price`.
    fn gas_price(&self) -> Option<U256>;
    /// Set `gas_price`.
    fn set_gas_price(&mut self, price: U256);
}

// TODO: Remove in favor of dyn trait upcasting (1.76+)
#[doc(hidden)]
impl<S: 'static> dyn Transaction<Signature = S> {
    pub fn __downcast_ref<T: std::any::Any>(&self) -> Option<&T> {
        if std::any::Any::type_id(self) == std::any::TypeId::of::<T>() {
            unsafe { Some(&*(self as *const _ as *const T)) }
        } else {
            None
        }
    }
}

/// Captures getters and setters common across EIP-1559 transactions across all networks
pub trait Eip1559Transaction: Transaction {
    /// Get `max_priority_fee_per_gas`.
    #[doc(alias = "max_tip")]
    fn max_priority_fee_per_gas(&self) -> U256;
    /// Set `max_priority_fee_per_gas`.
    #[doc(alias = "set_max_tip")]
    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: U256);

    /// Get `max_fee_per_gas`.
    fn max_fee_per_gas(&self) -> U256;
    /// Set `max_fee_per_gas`.
    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: U256);
}
