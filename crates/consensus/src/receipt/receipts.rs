use crate::receipt::{
    Eip2718EncodableReceipt, Eip658Value, RlpDecodableReceipt, RlpEncodableReceipt, TxReceipt,
};
use alloc::{vec, vec::Vec};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Bloom, Log};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};
use core::fmt;
use derive_more::{From, IntoIterator};

/// Receipt containing result of transaction execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "TransactionReceipt", alias = "TxReceipt")]
pub struct Receipt<T = Log> {
    /// If transaction is executed successfully.
    ///
    /// This is the `statusCode`
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub status: Eip658Value,
    /// Gas used
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub cumulative_gas_used: u128,
    /// Log send from contracts.
    pub logs: Vec<T>,
}

impl<T> Receipt<T>
where
    T: AsRef<Log>,
{
    /// Calculates [`Log`]'s bloom filter. this is slow operation and [ReceiptWithBloom] can
    /// be used to cache this value.
    pub fn bloom_slow(&self) -> Bloom {
        self.logs.iter().map(AsRef::as_ref).collect()
    }

    /// Calculates the bloom filter for the receipt and returns the [ReceiptWithBloom] container
    /// type.
    pub fn with_bloom(self) -> ReceiptWithBloom<Self> {
        ReceiptWithBloom { logs_bloom: self.bloom_slow(), receipt: self }
    }
}

impl<T> TxReceipt for Receipt<T>
where
    T: AsRef<Log> + Clone + fmt::Debug + PartialEq + Eq + Send + Sync,
{
    type Log = T;

    fn status_or_post_state(&self) -> Eip658Value {
        self.status
    }

    fn status(&self) -> bool {
        self.status.coerce_status()
    }

    fn bloom(&self) -> Bloom {
        self.bloom_slow()
    }

    fn cumulative_gas_used(&self) -> u128 {
        self.cumulative_gas_used
    }

    fn logs(&self) -> &[Self::Log] {
        &self.logs
    }
}

impl<T: Encodable> Receipt<T> {
    /// Returns length of RLP-encoded receipt fields with the given [`Bloom`] without an RLP header.
    pub fn rlp_encoded_fields_length_with_bloom(&self, bloom: &Bloom) -> usize {
        self.status.length()
            + self.cumulative_gas_used.length()
            + bloom.length()
            + self.logs.length()
    }

    /// RLP-encodes receipt fields with the given [`Bloom`] without an RLP header.
    pub fn rlp_encode_fields_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        self.status.encode(out);
        self.cumulative_gas_used.encode(out);
        bloom.encode(out);
        self.logs.encode(out);
    }

    /// Returns RLP header for this receipt encoding with the given [`Bloom`].
    pub fn rlp_header_with_bloom(&self, bloom: &Bloom) -> Header {
        Header { list: true, payload_length: self.rlp_encoded_fields_length_with_bloom(bloom) }
    }
}

impl<T: Encodable> RlpEncodableReceipt for Receipt<T> {
    fn rlp_encoded_length_with_bloom(&self, bloom: &Bloom) -> usize {
        self.rlp_header_with_bloom(bloom).length_with_payload()
    }

    fn rlp_encode_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        self.rlp_header_with_bloom(bloom).encode(out);
        self.rlp_encode_fields_with_bloom(bloom, out);
    }
}

impl<T: Decodable> Receipt<T> {
    /// RLP-decodes receipt's field with a [`Bloom`].
    ///
    /// Does not expect an RLP header.
    pub fn rlp_decode_fields_with_bloom(
        buf: &mut &[u8],
    ) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let status = Decodable::decode(buf)?;
        let cumulative_gas_used = Decodable::decode(buf)?;
        let logs_bloom = Decodable::decode(buf)?;
        let logs = Decodable::decode(buf)?;

        Ok(ReceiptWithBloom { receipt: Self { status, cumulative_gas_used, logs }, logs_bloom })
    }
}

impl<T: Decodable> RlpDecodableReceipt for Receipt<T> {
    fn rlp_decode_with_bloom(buf: &mut &[u8]) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let remaining = buf.len();

        let this = Self::rlp_decode_fields_with_bloom(buf)?;

        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        Ok(this)
    }
}

impl<T> From<ReceiptWithBloom<Self>> for Receipt<T> {
    /// Consume the structure, returning only the receipt
    fn from(receipt_with_bloom: ReceiptWithBloom<Self>) -> Self {
        receipt_with_bloom.receipt
    }
}

/// Receipt containing result of transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Default, From, IntoIterator)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Receipts<T> {
    /// A two-dimensional vector of [`Receipt`] instances.
    pub receipt_vec: Vec<Vec<T>>,
}

impl<T> Receipts<T> {
    /// Returns the length of the [`Receipts`] vector.
    pub fn len(&self) -> usize {
        self.receipt_vec.len()
    }

    /// Returns `true` if the [`Receipts`] vector is empty.
    pub fn is_empty(&self) -> bool {
        self.receipt_vec.is_empty()
    }

    /// Push a new vector of receipts into the [`Receipts`] collection.
    pub fn push(&mut self, receipts: Vec<T>) {
        self.receipt_vec.push(receipts);
    }
}

impl<T> From<Vec<T>> for Receipts<T> {
    fn from(block_receipts: Vec<T>) -> Self {
        Self { receipt_vec: vec![block_receipts] }
    }
}

impl<T> FromIterator<Vec<T>> for Receipts<T> {
    fn from_iter<I: IntoIterator<Item = Vec<T>>>(iter: I) -> Self {
        Self { receipt_vec: iter.into_iter().collect() }
    }
}

impl<T: Encodable> Encodable for Receipts<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.receipt_vec.encode(out)
    }

    fn length(&self) -> usize {
        self.receipt_vec.length()
    }
}

impl<T: Decodable> Decodable for Receipts<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self { receipt_vec: Decodable::decode(buf)? })
    }
}

/// [`Receipt`] with calculated bloom filter.
///
/// This convenience type allows us to lazily calculate the bloom filter for a
/// receipt, similar to [`Sealed`].
///
/// [`Sealed`]: crate::Sealed
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "TransactionReceiptWithBloom", alias = "TxReceiptWithBloom")]
pub struct ReceiptWithBloom<T = Receipt<Log>> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    /// The receipt.
    pub receipt: T,
    /// The bloom filter.
    pub logs_bloom: Bloom,
}

impl<R> TxReceipt for ReceiptWithBloom<R>
where
    R: TxReceipt,
{
    type Log = R::Log;

    fn status_or_post_state(&self) -> Eip658Value {
        self.receipt.status_or_post_state()
    }

    fn status(&self) -> bool {
        self.receipt.status()
    }

    fn bloom(&self) -> Bloom {
        self.logs_bloom
    }

    fn bloom_cheap(&self) -> Option<Bloom> {
        Some(self.logs_bloom)
    }

    fn cumulative_gas_used(&self) -> u128 {
        self.receipt.cumulative_gas_used()
    }

    fn logs(&self) -> &[Self::Log] {
        self.receipt.logs()
    }
}

impl<R> From<R> for ReceiptWithBloom<R>
where
    R: TxReceipt,
{
    fn from(receipt: R) -> Self {
        let logs_bloom = receipt.bloom();
        Self { logs_bloom, receipt }
    }
}

impl<R> ReceiptWithBloom<R> {
    /// Create new [ReceiptWithBloom]
    pub const fn new(receipt: R, logs_bloom: Bloom) -> Self {
        Self { receipt, logs_bloom }
    }

    /// Consume the structure, returning the receipt and the bloom filter
    pub fn into_components(self) -> (R, Bloom) {
        (self.receipt, self.logs_bloom)
    }
}

impl<R: RlpEncodableReceipt> Encodable for ReceiptWithBloom<R> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.receipt.rlp_encode_with_bloom(&self.logs_bloom, out);
    }

    fn length(&self) -> usize {
        self.receipt.rlp_encoded_length_with_bloom(&self.logs_bloom)
    }
}

impl<R: RlpDecodableReceipt> Decodable for ReceiptWithBloom<R> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        R::rlp_decode_with_bloom(buf)
    }
}

impl<R> Encodable2718 for ReceiptWithBloom<R>
where
    R: Eip2718EncodableReceipt + Send + Sync,
{
    fn type_flag(&self) -> Option<u8> {
        (!self.receipt.is_legacy()).then_some(self.receipt.ty())
    }

    fn encode_2718_len(&self) -> usize {
        self.receipt.eip2718_encoded_length_with_bloom(&self.logs_bloom)
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        self.receipt.eip2718_encode_with_bloom(&self.logs_bloom, out);
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, R> arbitrary::Arbitrary<'a> for ReceiptWithBloom<R>
where
    R: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self { receipt: R::arbitrary(u)?, logs_bloom: Bloom::arbitrary(u)? })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ReceiptEnvelope;
    use alloy_rlp::{Decodable, Encodable};

    const fn assert_tx_receipt<T: TxReceipt>() {}

    #[test]
    const fn assert_receipt() {
        assert_tx_receipt::<Receipt>();
        assert_tx_receipt::<ReceiptWithBloom<Receipt>>();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn root_vs_status() {
        let receipt = super::Receipt::<()> {
            status: super::Eip658Value::Eip658(true),
            cumulative_gas_used: 0,
            logs: Vec::new(),
        };

        let json = serde_json::to_string(&receipt).unwrap();
        assert_eq!(json, r#"{"status":"0x1","cumulativeGasUsed":"0x0","logs":[]}"#);

        let receipt = super::Receipt::<()> {
            status: super::Eip658Value::PostState(Default::default()),
            cumulative_gas_used: 0,
            logs: Vec::new(),
        };

        let json = serde_json::to_string(&receipt).unwrap();
        assert_eq!(
            json,
            r#"{"root":"0x0000000000000000000000000000000000000000000000000000000000000000","cumulativeGasUsed":"0x0","logs":[]}"#
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deser_pre658() {
        use alloy_primitives::b256;

        let json = r#"{"root":"0x284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10","cumulativeGasUsed":"0x0","logs":[]}"#;

        let receipt: super::Receipt<()> = serde_json::from_str(json).unwrap();

        assert_eq!(
            receipt.status,
            super::Eip658Value::PostState(b256!(
                "284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10"
            ))
        );
    }

    #[test]
    fn roundtrip_encodable_eip1559() {
        let receipts =
            Receipts { receipt_vec: vec![vec![ReceiptEnvelope::Eip1559(Default::default())]] };

        let mut out = vec![];
        receipts.encode(&mut out);

        let mut out = out.as_slice();
        let decoded = Receipts::<ReceiptEnvelope>::decode(&mut out).unwrap();

        assert_eq!(receipts, decoded);
    }
}
