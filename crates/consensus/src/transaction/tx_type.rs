//! Contains the Ethereum transaction type identifier.

use alloy_eips::{
    eip2718::{Eip2718Error, IsTyped2718},
    Typed2718,
};
use alloy_primitives::{U64, U8};
use alloy_rlp::{Decodable, Encodable};
use core::fmt;

/// The TxEnvelope enum represents all Ethereum transaction envelope types,
/// /// Its variants correspond to specific allowed transactions:
/// 1. Legacy (pre-EIP2718) [`TxLegacy`](crate::TxLegacy)
/// 2. EIP2930 (state access lists) [`TxEip2930`](crate::TxEip2930)
/// 3. EIP1559 [`TxEip1559`](crate::TxEip1559)
/// 4. EIP4844 [`TxEip4844Variant`](crate::TxEip4844Variant)
///
/// This type is generic over Eip4844 variant to support the following cases:
/// 1. Only-[`crate::TxEip4844`] transaction type, such transaction representation is returned by
///    RPC and stored by nodes internally.
/// 2. Only-[`crate::TxEip4844WithSidecar`] transactions which are broadcasted over the network,
///    submitted to RPC and stored in transaction pool.
/// 3. Dynamic [`TxEip4844Variant`](crate::TxEip4844Variant) transactions to support both of the
///    above cases via a single type.
///
/// Ethereum `TransactionType` flags as specified in EIPs [2718], [1559], [2930], [4844], and
/// [7702].
///
/// [2718]: https://eips.ethereum.org/EIPS/eip-2718
/// [1559]: https://eips.ethereum.org/EIPS/eip-1559
/// [2930]: https://eips.ethereum.org/EIPS/eip-2930
/// [4844]: https://eips.ethereum.org/EIPS/eip-4844
/// [7702]: https://eips.ethereum.org/EIPS/eip-7702
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "U8", try_from = "U64"))]
#[doc(alias = "TransactionType")]
pub enum TxType {
    /// Legacy transaction type.
    #[default]
    Legacy = 0,
    /// EIP-2930 transaction type.
    Eip2930 = 1,
    /// EIP-1559 transaction type.
    Eip1559 = 2,
    /// EIP-4844 transaction type.
    Eip4844 = 3,
    /// EIP-7702 transaction type.
    Eip7702 = 4,
}

impl From<TxType> for u8 {
    fn from(value: TxType) -> Self {
        value as Self
    }
}

impl From<TxType> for U8 {
    fn from(tx_type: TxType) -> Self {
        Self::from(u8::from(tx_type))
    }
}

impl TxType {
    /// Returns true if the transaction type is Legacy.
    #[inline]
    pub const fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy)
    }

    /// Returns true if the transaction type is EIP-2930.
    #[inline]
    pub const fn is_eip2930(&self) -> bool {
        matches!(self, Self::Eip2930)
    }

    /// Returns true if the transaction type is EIP-1559.
    #[inline]
    pub const fn is_eip1559(&self) -> bool {
        matches!(self, Self::Eip1559)
    }

    /// Returns true if the transaction type is EIP-4844.
    #[inline]
    pub const fn is_eip4844(&self) -> bool {
        matches!(self, Self::Eip4844)
    }

    /// Returns true if the transaction type is EIP-7702.
    #[inline]
    pub const fn is_eip7702(&self) -> bool {
        matches!(self, Self::Eip7702)
    }

    /// Returns true if the transaction type has dynamic fee.
    #[inline]
    pub const fn is_dynamic_fee(&self) -> bool {
        matches!(self, Self::Eip1559 | Self::Eip4844 | Self::Eip7702)
    }
}

impl IsTyped2718 for TxType {
    fn is_type(type_id: u8) -> bool {
        matches!(type_id, 0x0..=0x04)
    }
}

impl fmt::Display for TxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Legacy => write!(f, "Legacy"),
            Self::Eip2930 => write!(f, "EIP-2930"),
            Self::Eip1559 => write!(f, "EIP-1559"),
            Self::Eip4844 => write!(f, "EIP-4844"),
            Self::Eip7702 => write!(f, "EIP-7702"),
        }
    }
}

impl PartialEq<u8> for TxType {
    fn eq(&self, other: &u8) -> bool {
        (*self as u8) == *other
    }
}

impl PartialEq<TxType> for u8 {
    fn eq(&self, other: &TxType) -> bool {
        *self == *other as Self
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for TxType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(u.int_in_range(0u8..=4)?.try_into().unwrap())
    }
}

impl TryFrom<u8> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Legacy,
            1 => Self::Eip2930,
            2 => Self::Eip1559,
            3 => Self::Eip4844,
            4 => Self::Eip7702,
            _ => return Err(Eip2718Error::UnexpectedType(value)),
        })
    }
}

impl TryFrom<u64> for TxType {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let err = || "invalid tx type";
        let value: u8 = value.try_into().map_err(|_| err())?;
        Self::try_from(value).map_err(|_| err())
    }
}

impl TryFrom<U8> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: U8) -> Result<Self, Self::Error> {
        value.to::<u8>().try_into()
    }
}

impl TryFrom<U64> for TxType {
    type Error = &'static str;

    fn try_from(value: U64) -> Result<Self, Self::Error> {
        value.to::<u64>().try_into()
    }
}

impl Encodable for TxType {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        (*self as u8).encode(out);
    }

    fn length(&self) -> usize {
        1
    }
}

impl Decodable for TxType {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let ty = u8::decode(buf)?;
        Self::try_from(ty).map_err(|_| alloy_rlp::Error::Custom("invalid transaction type"))
    }
}

impl Typed2718 for TxType {
    fn ty(&self) -> u8 {
        (*self).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_u8_id() {
        assert_eq!(TxType::Legacy, TxType::Legacy as u8);
        assert_eq!(TxType::Eip2930, TxType::Eip2930 as u8);
        assert_eq!(TxType::Eip1559, TxType::Eip1559 as u8);
        assert_eq!(TxType::Eip7702, TxType::Eip7702 as u8);
        assert_eq!(TxType::Eip4844, TxType::Eip4844 as u8);
    }
}
