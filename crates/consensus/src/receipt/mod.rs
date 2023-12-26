use alloy_network::{Decodable2718, Eip2718Error, Encodable2718};
use alloy_primitives::{Bloom, Log};
use alloy_rlp::{length_of_length, BufMut, Decodable, Encodable};

use crate::TxType;

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
}

impl ReceiptEnvelope {
    /// Return the [`TxType`] of the inner receipt.
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) | Self::TaggedLegacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
        }
    }

    /// Return the inner receipt with bloom. Currently this is infallible,
    /// however, future receipt types may be added.
    pub const fn as_receipt_with_bloom(&self) -> Option<&ReceiptWithBloom> {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) | Self::Eip2930(t) | Self::Eip1559(t) => {
                Some(t)
            }
        }
    }

    /// Return the inner receipt. Currently this is infallible, however, future
    /// receipt types may be added.
    pub const fn as_receipt(&self) -> Option<&Receipt> {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) | Self::Eip2930(t) | Self::Eip1559(t) => {
                Some(&t.receipt)
            }
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
        }
    }

    fn encode_2718_len(&self) -> usize {
        self.inner_length() + 1
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
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, alloy_network::Eip2718Error> {
        let receipt = Decodable::decode(buf)?;
        match ty.try_into()? {
            TxType::Legacy => Ok(Self::TaggedLegacy(receipt)),
            TxType::Eip2930 => Ok(Self::Eip2930(receipt)),
            TxType::Eip1559 => Ok(Self::Eip1559(receipt)),
        }
    }

    fn fallback_decode(buf: &mut &[u8]) -> Result<Self, alloy_network::Eip2718Error> {
        Ok(Self::Legacy(Decodable::decode(buf)?))
    }
}

/// Receipt containing result of transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Receipt {
    /// If transaction is executed successfully.
    ///
    /// This is the `statusCode`
    pub success: bool,
    /// Gas used
    pub cumulative_gas_used: u64,
    /// Log send from contracts.
    pub logs: Vec<Log>,
}

impl Receipt {
    /// Calculates [`Log`]'s bloom filter. this is slow operation and [ReceiptWithBloom] can
    /// be used to cache this value.
    pub fn bloom_slow(&self) -> Bloom {
        self.logs.iter().collect()
    }

    /// Calculates the bloom filter for the receipt and returns the [ReceiptWithBloom] container
    /// type.
    pub fn with_bloom(self) -> ReceiptWithBloom {
        self.into()
    }
}

impl alloy_network::Receipt for Receipt {
    fn success(&self) -> bool {
        self.success
    }

    fn bloom(&self) -> Bloom {
        self.bloom_slow()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.cumulative_gas_used
    }

    fn logs(&self) -> &[Log] {
        &self.logs
    }
}

/// [`Receipt`] with calculated bloom filter.
///
/// This convenience type allows us to lazily calculate the bloom filter for a
/// receipt, similar to [`Sealed`].
///
/// [`Sealed`]: ::alloy_network::Sealed
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ReceiptWithBloom {
    /// The receipt.
    pub receipt: Receipt,
    /// The bloom filter.
    pub bloom: Bloom,
}

impl From<Receipt> for ReceiptWithBloom {
    fn from(receipt: Receipt) -> Self {
        let bloom = receipt.bloom_slow();
        ReceiptWithBloom { receipt, bloom }
    }
}

impl ReceiptWithBloom {
    /// Create new [ReceiptWithBloom]
    pub const fn new(receipt: Receipt, bloom: Bloom) -> Self {
        Self { receipt, bloom }
    }

    /// Consume the structure, returning only the receipt
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_receipt(self) -> Receipt {
        self.receipt
    }

    /// Consume the structure, returning the receipt and the bloom filter
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_components(self) -> (Receipt, Bloom) {
        (self.receipt, self.bloom)
    }

    fn payload_len(&self) -> usize {
        self.receipt.success.length()
            + self.receipt.cumulative_gas_used.length()
            + self.bloom.length()
            + self.receipt.logs.length()
    }

    /// Returns the rlp header for the receipt payload.
    fn receipt_rlp_header(&self) -> alloy_rlp::Header {
        alloy_rlp::Header { list: true, payload_length: self.payload_len() }
    }

    /// Encodes the receipt data.
    fn encode_fields(&self, out: &mut dyn BufMut) {
        self.receipt_rlp_header().encode(out);
        self.receipt.success.encode(out);
        self.receipt.cumulative_gas_used.encode(out);
        self.bloom.encode(out);
        self.receipt.logs.encode(out);
    }

    /// Decodes the receipt payload
    fn decode_receipt(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let b: &mut &[u8] = &mut &**buf;
        let rlp_head = alloy_rlp::Header::decode(b)?;
        if !rlp_head.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let started_len = b.len();

        let success = Decodable::decode(b)?;
        let cumulative_gas_used = Decodable::decode(b)?;
        let bloom = Decodable::decode(b)?;
        let logs = Decodable::decode(b)?;

        let receipt = Receipt { success, cumulative_gas_used, logs };

        let this = Self { receipt, bloom };
        let consumed = started_len - b.len();
        if consumed != rlp_head.payload_length {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        *buf = *b;
        Ok(this)
    }
}

impl alloy_network::Receipt for ReceiptWithBloom {
    fn success(&self) -> bool {
        self.receipt.success
    }

    fn bloom(&self) -> Bloom {
        self.bloom
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.receipt.cumulative_gas_used
    }

    fn logs(&self) -> &[Log] {
        &self.receipt.logs
    }
}

impl alloy_rlp::Encodable for ReceiptWithBloom {
    fn encode(&self, out: &mut dyn BufMut) {
        self.encode_fields(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.receipt.success.length()
            + self.receipt.cumulative_gas_used.length()
            + self.bloom.length()
            + self.receipt.logs.length();
        payload_length + length_of_length(payload_length)
    }
}

impl alloy_rlp::Decodable for ReceiptWithBloom {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::decode_receipt(buf)
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for Receipt {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let success = bool::arbitrary(u)?;
        let cumulative_gas_used = u64::arbitrary(u)?;
        let logs = Vec::<Log>::arbitrary(u)?;

        // Only receipts for deposit transactions may contain a deposit nonce
        #[cfg(feature = "optimism")]
        let deposit_nonce =
            if tx_type == TxType::DEPOSIT { Option::<u64>::arbitrary(u)? } else { None };

        Ok(Self {
            success,
            cumulative_gas_used,
            logs,
            #[cfg(feature = "optimism")]
            deposit_nonce,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_network::Encodable2718;
    use alloy_primitives::{address, b256, bytes, hex, Bytes, LogData};

    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    #[test]
    fn encode_legacy_receipt() {
        let expected = hex!("f901668001b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f85ff85d940000000000000000000000000000000000000011f842a0000000000000000000000000000000000000000000000000000000000000deada0000000000000000000000000000000000000000000000000000000000000beef830100ff");

        let mut data = vec![];
        let receipt =
            ReceiptEnvelope::Legacy(ReceiptWithBloom {
                receipt: Receipt {
                    cumulative_gas_used: 0x1u64,
                    logs: vec![Log {
                        address: address!("0000000000000000000000000000000000000011"),
                        data: LogData::new_unchecked(
                            vec![
                    b256!("000000000000000000000000000000000000000000000000000000000000dead"),
                    b256!("000000000000000000000000000000000000000000000000000000000000beef"),
                ],
                            bytes!("0100ff"),
                        ),
                    }],
                    success: false,
                },
                bloom: [0; 256].into(),
            });

        receipt.network_encode(&mut data);

        // check that the rlp length equals the length of the expected rlp
        assert_eq!(receipt.length(), expected.len());
        assert_eq!(data, expected);
    }

    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    #[test]
    fn decode_legacy_receipt() {
        let data = hex!("f901668001b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f85ff85d940000000000000000000000000000000000000011f842a0000000000000000000000000000000000000000000000000000000000000deada0000000000000000000000000000000000000000000000000000000000000beef830100ff");

        // EIP658Receipt
        let expected =
            ReceiptWithBloom {
                receipt: Receipt {
                    cumulative_gas_used: 0x1u64,
                    logs: vec![Log {
                        address: address!("0000000000000000000000000000000000000011"),
                        data: LogData::new_unchecked(
                            vec![
                        b256!("000000000000000000000000000000000000000000000000000000000000dead"),
                        b256!("000000000000000000000000000000000000000000000000000000000000beef"),
                    ],
                            bytes!("0100ff"),
                        ),
                    }],
                    success: false,
                },
                bloom: [0; 256].into(),
            };

        let receipt = ReceiptWithBloom::decode(&mut &data[..]).unwrap();
        assert_eq!(receipt, expected);
    }

    #[test]
    fn gigantic_receipt() {
        let receipt = Receipt {
            cumulative_gas_used: 16747627,
            success: true,
            logs: vec![
                Log {
                    address: address!("4bf56695415f725e43c3e04354b604bcfb6dfb6e"),
                    data: LogData::new_unchecked(
                        vec![b256!(
                            "c69dc3d7ebff79e41f525be431d5cd3cc08f80eaf0f7819054a726eeb7086eb9"
                        )],
                        Bytes::from(vec![1; 0xffffff]),
                    ),
                },
                Log {
                    address: address!("faca325c86bf9c2d5b413cd7b90b209be92229c2"),
                    data: LogData::new_unchecked(
                        vec![b256!(
                            "8cca58667b1e9ffa004720ac99a3d61a138181963b294d270d91c53d36402ae2"
                        )],
                        Bytes::from(vec![1; 0xffffff]),
                    ),
                },
            ],
        }
        .with_bloom();

        let mut data = vec![];

        receipt.encode(&mut data);
        let decoded = ReceiptWithBloom::decode(&mut &data[..]).unwrap();

        // receipt.clone().to_compact(&mut data);
        // let (decoded, _) = Receipt::from_compact(&data[..], data.len());
        assert_eq!(decoded, receipt);
    }
}
