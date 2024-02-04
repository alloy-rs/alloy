use alloy_network::Transaction;
use alloy_primitives::Signature;
use alloy_rlp::Encodable;

use crate::{TxEip1559, TxEip2930, TxEnvelope, TxLegacy, TxType};

/// The TypedTransactionRequest enum represents all Ethereum transaction request types.
///
/// Its variants correspond to specific allowed transactions:
/// 1. Legacy (pre-EIP2718) [`TxLegacy`]
/// 2. EIP2930 (state access lists) [`TxEip2930`]
/// 3. EIP1559 [`TxEip1559`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypedTransactionRequest {
    /// Legacy transaction
    Legacy(TxLegacy),
    /// EIP-2930 transaction
    Eip2930(TxEip2930),
    /// EIP-1559 transaction
    Eip1559(TxEip1559),
}

impl From<TxLegacy> for TypedTransactionRequest {
    fn from(tx: TxLegacy) -> Self {
        Self::Legacy(tx)
    }
}

impl From<TxEip2930> for TypedTransactionRequest {
    fn from(tx: TxEip2930) -> Self {
        Self::Eip2930(tx)
    }
}

impl From<TxEip1559> for TypedTransactionRequest {
    fn from(tx: TxEip1559) -> Self {
        Self::Eip1559(tx)
    }
}

impl TypedTransactionRequest {
    /// Return the [`TxType`] of the inner txn.
    pub fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
        }
    }

    /// Consumes the type and returns the EIP-2718 enveloped transaction with the given signature.
    pub fn to_enveloped(self, sig: Signature) -> TxEnvelope {
        match self {
            Self::Legacy(tx) => crate::TxEnvelope::Legacy(tx.into_signed(sig)),
            Self::Eip2930(tx) => crate::TxEnvelope::Eip2930(tx.into_signed(sig)),
            Self::Eip1559(tx) => crate::TxEnvelope::Eip1559(tx.into_signed(sig)),
        }
    }

    /// Sets the chain ID for the transaction.
    pub fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
        match self {
            Self::Legacy(tx) => tx.set_chain_id(chain_id),
            Self::Eip2930(tx) => tx.set_chain_id(chain_id),
            Self::Eip1559(tx) => tx.set_chain_id(chain_id),
        }
    }
}

impl Encodable for TypedTransactionRequest {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(tx) => tx.encode(out),
            Self::Eip2930(tx) => tx.encode(out),
            Self::Eip1559(tx) => tx.encode(out),
        }
    }
}

impl Transaction for TypedTransactionRequest {
    type Signature = Signature;

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(tx) => tx.encode_for_signing(out),
            Self::Eip2930(tx) => tx.encode_for_signing(out),
            Self::Eip1559(tx) => tx.encode_for_signing(out),
        }
    }

    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        match self {
            Self::Legacy(tx) => tx.chain_id(),
            Self::Eip2930(tx) => tx.chain_id(),
            Self::Eip1559(tx) => tx.chain_id(),
        }
    }

    fn decode_signed(_buf: &mut &[u8]) -> alloy_rlp::Result<alloy_network::Signed<Self>>
    where
        Self: Sized,
    {
        unimplemented!();
    }

    fn encode_signed(&self, signature: &Signature, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(tx) => tx.encode_signed(signature, out),
            Self::Eip2930(tx) => tx.encode_signed(signature, out),
            Self::Eip1559(tx) => tx.encode_signed(signature, out),
        }
    }

    fn encoded_for_signing(&self) -> Vec<u8> {
        match self {
            Self::Legacy(tx) => tx.encoded_for_signing(),
            Self::Eip2930(tx) => tx.encoded_for_signing(),
            Self::Eip1559(tx) => tx.encoded_for_signing(),
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.gas_limit(),
            Self::Eip2930(tx) => tx.gas_limit(),
            Self::Eip1559(tx) => tx.gas_limit(),
        }
    }

    fn gas_price(&self) -> Option<alloy_primitives::U256> {
        match self {
            Self::Legacy(tx) => tx.gas_price(),
            Self::Eip2930(tx) => tx.gas_price(),
            Self::Eip1559(tx) => tx.gas_price(),
        }
    }

    fn input(&self) -> &[u8] {
        match self {
            Self::Legacy(tx) => tx.input(),
            Self::Eip2930(tx) => tx.input(),
            Self::Eip1559(tx) => tx.input(),
        }
    }

    fn input_mut(&mut self) -> &mut alloy_primitives::Bytes {
        match self {
            Self::Legacy(tx) => tx.input_mut(),
            Self::Eip2930(tx) => tx.input_mut(),
            Self::Eip1559(tx) => tx.input_mut(),
        }
    }

    fn into_signed(self, _signature: Signature) -> alloy_network::Signed<Self, Self::Signature>
    where
        Self: Sized,
    {
        unimplemented!();
    }

    fn nonce(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.nonce(),
            Self::Eip2930(tx) => tx.nonce(),
            Self::Eip1559(tx) => tx.nonce(),
        }
    }

    fn payload_len_for_signature(&self) -> usize {
        match self {
            Self::Legacy(tx) => tx.payload_len_for_signature(),
            Self::Eip2930(tx) => tx.payload_len_for_signature(),
            Self::Eip1559(tx) => tx.payload_len_for_signature(),
        }
    }

    fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
        match self {
            Self::Legacy(tx) => tx.set_chain_id(chain_id),
            Self::Eip2930(tx) => tx.set_chain_id(chain_id),
            Self::Eip1559(tx) => tx.set_chain_id(chain_id),
        }
    }

    fn set_gas_limit(&mut self, limit: u64) {
        match self {
            Self::Legacy(tx) => tx.set_gas_limit(limit),
            Self::Eip2930(tx) => tx.set_gas_limit(limit),
            Self::Eip1559(tx) => tx.set_gas_limit(limit),
        }
    }

    fn set_gas_price(&mut self, price: alloy_primitives::U256) {
        match self {
            Self::Legacy(tx) => tx.set_gas_price(price),
            Self::Eip2930(tx) => tx.set_gas_price(price),
            Self::Eip1559(tx) => tx.set_gas_price(price),
        }
    }

    fn set_input(&mut self, data: alloy_primitives::Bytes) {
        match self {
            Self::Legacy(tx) => tx.set_input(data),
            Self::Eip2930(tx) => tx.set_input(data),
            Self::Eip1559(tx) => tx.set_input(data),
        }
    }

    fn set_nonce(&mut self, nonce: u64) {
        match self {
            Self::Legacy(tx) => tx.set_nonce(nonce),
            Self::Eip2930(tx) => tx.set_nonce(nonce),
            Self::Eip1559(tx) => tx.set_nonce(nonce),
        }
    }

    fn set_to(&mut self, to: alloy_network::TxKind) {
        match self {
            Self::Legacy(tx) => tx.set_to(to),
            Self::Eip2930(tx) => tx.set_to(to),
            Self::Eip1559(tx) => tx.set_to(to),
        }
    }

    fn set_value(&mut self, value: alloy_primitives::U256) {
        match self {
            Self::Legacy(tx) => tx.set_value(value),
            Self::Eip2930(tx) => tx.set_value(value),
            Self::Eip1559(tx) => tx.set_value(value),
        }
    }

    fn signature_hash(&self) -> alloy_primitives::B256 {
        match self {
            Self::Legacy(tx) => tx.signature_hash(),
            Self::Eip2930(tx) => tx.signature_hash(),
            Self::Eip1559(tx) => tx.signature_hash(),
        }
    }

    fn to(&self) -> alloy_network::TxKind {
        match self {
            Self::Legacy(tx) => tx.to(),
            Self::Eip2930(tx) => tx.to(),
            Self::Eip1559(tx) => tx.to(),
        }
    }

    fn value(&self) -> alloy_primitives::U256 {
        match self {
            Self::Legacy(tx) => tx.value(),
            Self::Eip2930(tx) => tx.value(),
            Self::Eip1559(tx) => tx.value(),
        }
    }
}
