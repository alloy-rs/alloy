use crate::{TxEip1559, TxEip2930, TxLegacy};
use alloy_eips::eip2718::{Decodable2718, Eip2718Error, Encodable2718};
use alloy_network::Signed;
use alloy_rlp::{length_of_length, Decodable, Encodable};

/// Ethereum `TransactionType` flags as specified in EIPs [2718], [1559], and
/// [2930].
///
/// [2718]: https://eips.ethereum.org/EIPS/eip-2718
/// [1559]: https://eips.ethereum.org/EIPS/eip-1559
/// [2930]: https://eips.ethereum.org/EIPS/eip-2930
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum TxType {
    /// Wrapped legacy transaction type.
    Legacy = 0,
    /// EIP-2930 transaction type.
    Eip2930 = 1,
    /// EIP-1559 transaction type.
    Eip1559 = 2,
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for TxType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range(0..=2)? {
            0 => TxType::Legacy,
            1 => TxType::Eip2930,
            2 => TxType::Eip1559,
            _ => unreachable!(),
        })
    }
}

impl TryFrom<u8> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: repr(u8) with explicit discriminant
            ..=2 => Ok(unsafe { std::mem::transmute(value) }),
            _ => Err(Eip2718Error::UnexpectedType(value)),
        }
    }
}

/// The Ethereum [EIP-2718] Transaction Envelope.
///
/// # Note:
///
/// This enum distinguishes between tagged and untagged legacy transactions, as
/// the in-protocol merkle tree may commit to EITHER 0-prefixed or raw.
/// Therefore we must ensure that encoding returns the precise byte-array that
/// was decoded, preserving the presence or absence of the `TransactionType`
/// flag.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxEnvelope {
    /// An untagged [`TxLegacy`].
    Legacy(Signed<TxLegacy>),
    /// A [`TxLegacy`] tagged with type 0.
    TaggedLegacy(Signed<TxLegacy>),
    /// A [`TxEip2930`].
    Eip2930(Signed<TxEip2930>),
    /// A [`TxEip1559`].
    Eip1559(Signed<TxEip1559>),
}

impl From<Signed<TxEip2930>> for TxEnvelope {
    fn from(v: Signed<TxEip2930>) -> Self {
        Self::Eip2930(v)
    }
}

impl From<Signed<TxEip1559>> for TxEnvelope {
    fn from(v: Signed<TxEip1559>) -> Self {
        Self::Eip1559(v)
    }
}

impl TxEnvelope {
    /// Return the [`TxType`] of the inner txn.
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) | Self::TaggedLegacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
        }
    }

    /// Return the length of the inner txn.
    pub fn inner_length(&self) -> usize {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) => t.length(),
            Self::Eip2930(t) => t.length(),
            Self::Eip1559(t) => t.length(),
        }
    }

    /// Return the RLP payload length of the network-serialized wrapper
    fn rlp_payload_length(&self) -> usize {
        if let Self::Legacy(t) = self {
            return t.length();
        }
        // length of inner tx body
        let inner_length = self.inner_length();
        // with tx type byte
        inner_length + 1
    }
}

impl Encodable for TxEnvelope {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.network_encode(out)
    }

    fn length(&self) -> usize {
        let mut payload_length = self.rlp_payload_length();
        if !self.is_legacy() {
            payload_length += length_of_length(payload_length);
        }
        payload_length
    }
}

impl Decodable for TxEnvelope {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        match Self::network_decode(buf) {
            Ok(t) => Ok(t),
            Err(Eip2718Error::RlpError(e)) => Err(e),
            Err(_) => Err(alloy_rlp::Error::Custom("Unexpected type")),
        }
    }
}

impl Decodable2718 for TxEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        match ty.try_into()? {
            TxType::Legacy => Ok(Self::TaggedLegacy(Decodable::decode(buf)?)),
            TxType::Eip2930 => Ok(Self::Eip2930(Decodable::decode(buf)?)),
            TxType::Eip1559 => Ok(Self::Eip1559(Decodable::decode(buf)?)),
        }
    }

    fn fallback_decode(buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        Ok(TxEnvelope::Legacy(Decodable::decode(buf)?))
    }
}

impl Encodable2718 for TxEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            Self::Legacy(_) => None,
            Self::TaggedLegacy(_) => Some(TxType::Legacy as u8),
            Self::Eip2930(_) => Some(TxType::Eip2930 as u8),
            Self::Eip1559(_) => Some(TxType::Eip1559 as u8),
        }
    }

    fn encode_2718_len(&self) -> usize {
        self.inner_length() + !self.is_legacy() as usize
    }

    fn encode_2718(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            TxEnvelope::Legacy(tx) => tx.encode(out),
            TxEnvelope::TaggedLegacy(tx) => {
                out.put_u8(TxType::Legacy as u8);
                tx.encode(out);
            }
            TxEnvelope::Eip2930(tx) => {
                out.put_u8(TxType::Eip2930 as u8);
                tx.encode(out);
            }
            TxEnvelope::Eip1559(tx) => {
                out.put_u8(TxType::Eip1559 as u8);
                tx.encode(out);
            }
        }
    }
}
