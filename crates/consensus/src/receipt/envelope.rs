use crate::{Receipt, ReceiptWithBloom, TxType};
use alloy_eips::eip2718::{Decodable2718, Eip2718Error, Encodable2718};
use alloy_rlp::{length_of_length, BufMut, Decodable, Encodable};

/// Receipt envelope, as defined in [EIP-2718].
///
/// This enum distinguishes between tagged and untagged legacy receipts, as the
/// in-protocol merkle tree may commit to EITHER 0-prefixed or raw. Therefore
/// we must ensure that encoding returns the precise byte-array that was
/// decoded, preserving the presence or absence of the `TransactionType` flag.
///
/// Transaction receipt payloads are specified in their respective EIPs.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptEnvelope {
    /// Receipt envelope with no type flag.
    Legacy(ReceiptWithBloom),
    /// Receipt envelope with type flag 0.
    TaggedLegacy(ReceiptWithBloom),
    /// Receipt envelope with type flag 1, containing a [EIP-2930] receipt.
    ///
    /// [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    Eip2930(ReceiptWithBloom),
    /// Receipt envelope with type flag 2, containing a [EIP-1559] receipt.
    ///
    /// [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    Eip1559(ReceiptWithBloom),
    #[cfg(feature = "kzg")]
    /// Receipt envelope with type flag 2, containing a [EIP-4844] receipt.
    ///
    /// [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    Eip4844(ReceiptWithBloom),
}

impl ReceiptEnvelope {
    /// Return the [`TxType`] of the inner receipt.
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) | Self::TaggedLegacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
            #[cfg(feature = "kzg")]
            Self::Eip4844(_) => TxType::Eip4844,
        }
    }

    /// Return the inner receipt with bloom. Currently this is infallible,
    /// however, future receipt types may be added.
    pub const fn as_receipt_with_bloom(&self) -> Option<&ReceiptWithBloom> {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) | Self::Eip2930(t) | Self::Eip1559(t) => {
                Some(t)
            }
            #[cfg(feature = "kzg")]
            Self::Eip4844(t) => Some(t),
        }
    }

    /// Return the inner receipt. Currently this is infallible, however, future
    /// receipt types may be added.
    pub const fn as_receipt(&self) -> Option<&Receipt> {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) | Self::Eip2930(t) | Self::Eip1559(t) => {
                Some(&t.receipt)
            }
            #[cfg(feature = "kzg")]
            Self::Eip4844(t) => Some(&t.receipt),
        }
    }

    /// Get the length of the inner receipt in the 2718 encoding.
    pub fn inner_length(&self) -> usize {
        self.as_receipt_with_bloom().unwrap().length()
    }

    /// Calculate the length of the rlp payload of the network encoded receipt.
    pub fn rlp_payload_length(&self) -> usize {
        let length = self.as_receipt_with_bloom().unwrap().length();
        match self {
            Self::Legacy(_) => length,
            _ => length + 1,
        }
    }
}

impl Encodable for ReceiptEnvelope {
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

impl Decodable for ReceiptEnvelope {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        match Self::network_decode(buf) {
            Ok(t) => Ok(t),
            Err(Eip2718Error::RlpError(e)) => Err(e),
            Err(_) => Err(alloy_rlp::Error::Custom("Unexpected type")),
        }
    }
}

impl Encodable2718 for ReceiptEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            Self::Legacy(_) => None,
            Self::TaggedLegacy(_) => Some(TxType::Legacy as u8),
            Self::Eip2930(_) => Some(TxType::Eip2930 as u8),
            Self::Eip1559(_) => Some(TxType::Eip1559 as u8),
            #[cfg(feature = "kzg")]
            Self::Eip4844(_) => Some(TxType::Eip4844 as u8),
        }
    }

    fn encode_2718_len(&self) -> usize {
        self.inner_length() + !self.is_legacy() as usize
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        match self.type_flag() {
            None => {}
            Some(ty) => out.put_u8(ty),
        }
        self.as_receipt_with_bloom().unwrap().encode(out);
    }
}

impl Decodable2718 for ReceiptEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        let receipt = Decodable::decode(buf)?;
        match ty.try_into()? {
            TxType::Legacy => Ok(Self::TaggedLegacy(receipt)),
            TxType::Eip2930 => Ok(Self::Eip2930(receipt)),
            TxType::Eip1559 => Ok(Self::Eip1559(receipt)),
            #[cfg(feature = "kzg")]
            TxType::Eip4844 => Ok(Self::Eip4844(receipt)),
        }
    }

    fn fallback_decode(buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        Ok(Self::Legacy(Decodable::decode(buf)?))
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for ReceiptEnvelope {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let tx_type = u.int_in_range(-1..=2)?;
        let receipt = Receipt::arbitrary(u)?.with_bloom();

        match tx_type {
            -1 => Ok(Self::Legacy(receipt)),
            0 => Ok(Self::TaggedLegacy(receipt)),
            1 => Ok(Self::Eip2930(receipt)),
            2 => Ok(Self::Eip1559(receipt)),
            _ => unreachable!(),
        }
    }
}
