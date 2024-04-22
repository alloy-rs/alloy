use crate::{
    Transaction, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar,
    TxEnvelope, TxLegacy, TxType,
};
use alloy_primitives::TxKind;

/// The TypedTransaction enum represents all Ethereum transaction request types.
///
/// Its variants correspond to specific allowed transactions:
/// 1. Legacy (pre-EIP2718) [`TxLegacy`]
/// 2. EIP2930 (state access lists) [`TxEip2930`]
/// 3. EIP1559 [`TxEip1559`]
/// 4. EIP4844 [`TxEip4844Variant`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum TypedTransaction {
    /// Legacy transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x00", alias = "0x0"))]
    Legacy(TxLegacy),
    /// EIP-2930 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x01", alias = "0x1"))]
    Eip2930(TxEip2930),
    /// EIP-1559 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x02", alias = "0x2"))]
    Eip1559(TxEip1559),
    /// EIP-4844 transaction
    #[cfg_attr(feature = "serde", serde(rename = "0x03", alias = "0x3"))]
    Eip4844(TxEip4844Variant),
}

impl From<TxLegacy> for TypedTransaction {
    fn from(tx: TxLegacy) -> Self {
        Self::Legacy(tx)
    }
}

impl From<TxEip2930> for TypedTransaction {
    fn from(tx: TxEip2930) -> Self {
        Self::Eip2930(tx)
    }
}

impl From<TxEip1559> for TypedTransaction {
    fn from(tx: TxEip1559) -> Self {
        Self::Eip1559(tx)
    }
}

impl From<TxEip4844Variant> for TypedTransaction {
    fn from(tx: TxEip4844Variant) -> Self {
        Self::Eip4844(tx)
    }
}

impl From<TxEip4844> for TypedTransaction {
    fn from(tx: TxEip4844) -> Self {
        Self::Eip4844(tx.into())
    }
}

impl From<TxEip4844WithSidecar> for TypedTransaction {
    fn from(tx: TxEip4844WithSidecar) -> Self {
        Self::Eip4844(tx.into())
    }
}

impl From<TxEnvelope> for TypedTransaction {
    fn from(envelope: TxEnvelope) -> Self {
        match envelope {
            TxEnvelope::Legacy(tx) => Self::Legacy(tx.strip_signature()),
            TxEnvelope::Eip2930(tx) => Self::Eip2930(tx.strip_signature()),
            TxEnvelope::Eip1559(tx) => Self::Eip1559(tx.strip_signature()),
            TxEnvelope::Eip4844(tx) => Self::Eip4844(tx.strip_signature()),
        }
    }
}

impl TypedTransaction {
    /// Return the [`TxType`] of the inner txn.
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
            Self::Eip4844(_) => TxType::Eip4844,
        }
    }

    /// Return the inner legacy transaction if it exists.
    pub const fn legacy(&self) -> Option<&TxLegacy> {
        match self {
            Self::Legacy(tx) => Some(tx),
            _ => None,
        }
    }

    /// Return the inner EIP-2930 transaction if it exists.
    pub const fn eip2930(&self) -> Option<&TxEip2930> {
        match self {
            Self::Eip2930(tx) => Some(tx),
            _ => None,
        }
    }

    /// Return the inner EIP-1559 transaction if it exists.
    pub const fn eip1559(&self) -> Option<&TxEip1559> {
        match self {
            Self::Eip1559(tx) => Some(tx),
            _ => None,
        }
    }
}

impl Transaction for TypedTransaction {
    fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
        match self {
            Self::Legacy(tx) => tx.chain_id(),
            Self::Eip2930(tx) => tx.chain_id(),
            Self::Eip1559(tx) => tx.chain_id(),
            Self::Eip4844(tx) => tx.chain_id(),
        }
    }

    fn gas_limit(&self) -> u128 {
        match self {
            Self::Legacy(tx) => tx.gas_limit(),
            Self::Eip2930(tx) => tx.gas_limit(),
            Self::Eip1559(tx) => tx.gas_limit(),
            Self::Eip4844(tx) => tx.gas_limit(),
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            Self::Legacy(tx) => tx.gas_price(),
            Self::Eip2930(tx) => tx.gas_price(),
            Self::Eip1559(tx) => tx.gas_price(),
            Self::Eip4844(tx) => tx.gas_price(),
        }
    }

    fn input(&self) -> &[u8] {
        match self {
            Self::Legacy(tx) => tx.input(),
            Self::Eip2930(tx) => tx.input(),
            Self::Eip1559(tx) => tx.input(),
            Self::Eip4844(tx) => tx.input(),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.nonce(),
            Self::Eip2930(tx) => tx.nonce(),
            Self::Eip1559(tx) => tx.nonce(),
            Self::Eip4844(tx) => tx.nonce(),
        }
    }

    fn to(&self) -> TxKind {
        match self {
            Self::Legacy(tx) => tx.to(),
            Self::Eip2930(tx) => tx.to(),
            Self::Eip1559(tx) => tx.to(),
            Self::Eip4844(tx) => tx.to(),
        }
    }

    fn value(&self) -> alloy_primitives::U256 {
        match self {
            Self::Legacy(tx) => tx.value(),
            Self::Eip2930(tx) => tx.value(),
            Self::Eip1559(tx) => tx.value(),
            Self::Eip4844(tx) => tx.value(),
        }
    }
}
