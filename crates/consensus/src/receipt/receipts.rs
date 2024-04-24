use core::borrow::Borrow;

use super::TxReceipt;
use alloy_primitives::{Bloom, Log};
use alloy_rlp::{length_of_length, BufMut, Decodable, Encodable};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Receipt containing result of transaction execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Receipt<T = Log> {
    /// If transaction is executed successfully.
    ///
    /// This is the `statusCode`
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity_bool"))]
    pub status: bool,
    /// Gas used
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u128_via_ruint"))]
    pub cumulative_gas_used: u128,
    /// Log send from contracts.
    pub logs: Vec<T>,
}

impl<T> Receipt<T>
where
    T: Borrow<Log>,
{
    /// Calculates [`Log`]'s bloom filter. this is slow operation and [ReceiptWithBloom] can
    /// be used to cache this value.
    pub fn bloom_slow(&self) -> Bloom {
        self.logs.iter().map(Borrow::borrow).collect()
    }

    /// Calculates the bloom filter for the receipt and returns the [ReceiptWithBloom] container
    /// type.
    pub fn with_bloom(self) -> ReceiptWithBloom<T> {
        self.into()
    }
}

impl<T> TxReceipt<T> for Receipt<T>
where
    T: Borrow<Log>,
{
    fn status(&self) -> bool {
        self.status
    }

    fn bloom(&self) -> Bloom {
        self.bloom_slow()
    }

    fn cumulative_gas_used(&self) -> u128 {
        self.cumulative_gas_used
    }

    fn logs(&self) -> &[T] {
        &self.logs
    }
}

/// [`Receipt`] with calculated bloom filter.
///
/// This convenience type allows us to lazily calculate the bloom filter for a
/// receipt, similar to [`Sealed`].
///
/// [`Sealed`]: crate::sealed::Sealed
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ReceiptWithBloom<T = Log> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    /// The receipt.
    pub receipt: Receipt<T>,
    /// The bloom filter.
    pub logs_bloom: Bloom,
}

impl<T> TxReceipt<T> for ReceiptWithBloom<T> {
    fn status(&self) -> bool {
        self.receipt.status
    }

    fn bloom(&self) -> Bloom {
        self.logs_bloom
    }

    fn bloom_cheap(&self) -> Option<Bloom> {
        Some(self.logs_bloom)
    }

    fn cumulative_gas_used(&self) -> u128 {
        self.receipt.cumulative_gas_used
    }

    fn logs(&self) -> &[T] {
        &self.receipt.logs
    }
}

impl<T> From<Receipt<T>> for ReceiptWithBloom<T>
where
    T: Borrow<Log>,
{
    fn from(receipt: Receipt<T>) -> Self {
        let bloom = receipt.bloom_slow();
        ReceiptWithBloom { receipt, logs_bloom: bloom }
    }
}

impl<T: Encodable> ReceiptWithBloom<T> {
    /// Returns the rlp header for the receipt payload.
    fn receipt_rlp_header(&self) -> alloy_rlp::Header {
        alloy_rlp::Header { list: true, payload_length: self.payload_len() }
    }

    /// Encodes the receipt data.
    fn encode_fields(&self, out: &mut dyn BufMut) {
        self.receipt_rlp_header().encode(out);
        self.receipt.status.encode(out);
        self.receipt.cumulative_gas_used.encode(out);
        self.logs_bloom.encode(out);
        self.receipt.logs.encode(out);
    }

    fn payload_len(&self) -> usize {
        self.receipt.status.length()
            + self.receipt.cumulative_gas_used.length()
            + self.logs_bloom.length()
            + self.receipt.logs.length()
    }
}

impl<T> ReceiptWithBloom<T> {
    /// Create new [ReceiptWithBloom]
    pub const fn new(receipt: Receipt<T>, bloom: Bloom) -> Self {
        Self { receipt, logs_bloom: bloom }
    }

    /// Consume the structure, returning only the receipt
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_receipt(self) -> Receipt<T> {
        self.receipt
    }

    /// Consume the structure, returning the receipt and the bloom filter
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_components(self) -> (Receipt<T>, Bloom) {
        (self.receipt, self.logs_bloom)
    }

    /// Decodes the receipt payload
    fn decode_receipt(buf: &mut &[u8]) -> alloy_rlp::Result<Self>
    where
        T: Decodable,
    {
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

        let receipt = Receipt { status: success, cumulative_gas_used, logs };

        let this = Self { receipt, logs_bloom: bloom };
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

impl<T: Encodable> Encodable for ReceiptWithBloom<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.encode_fields(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.receipt.status.length()
            + self.receipt.cumulative_gas_used.length()
            + self.logs_bloom.length()
            + self.receipt.logs.length();
        payload_length + length_of_length(payload_length)
    }
}

impl<T: Decodable> Decodable for ReceiptWithBloom<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::decode_receipt(buf)
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T> arbitrary::Arbitrary<'a> for ReceiptWithBloom<T>
where
    T: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self { receipt: Receipt::<T>::arbitrary(u)?, logs_bloom: Bloom::arbitrary(u)? })
    }
}
