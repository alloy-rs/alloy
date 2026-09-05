use crate::{
    Eip2718DecodableReceipt, Eip2718EncodableReceipt, Eip658Value, InMemorySize, Receipt,
    ReceiptWithBloom, RlpDecodableReceipt, RlpEncodableReceipt, TxReceipt, TxType,
};
use alloc::vec::Vec;
use alloy_eips::{
    eip2718::{
        Decodable2718, Eip2718Error, Eip2718Result, Encodable2718, IsTyped2718, EIP1559_TX_TYPE_ID,
        EIP2930_TX_TYPE_ID, EIP4844_TX_TYPE_ID, EIP7702_TX_TYPE_ID, LEGACY_TX_TYPE_ID,
    },
    eip8141::{constants::FRAME_TX_TYPE, FrameReceiptPayload, FrameStatus},
    Typed2718,
};
use alloy_primitives::{logs_bloom, Bloom, Log};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};
use core::fmt;

/// Receipt envelope, as defined in [EIP-2718].
///
/// Represents untagged legacy receipts and typed EIP-2718 variants. Binary decoding rejects a
/// literal `0x00` type prefix; legacy receipts use the untagged fallback encoding. JSON `type: 0x0`
/// is accepted as legacy.
///
/// Transaction receipt payloads are specified in their respective EIPs.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "type", bound(serialize = "T: serde::Serialize + AsRef<Log>"))
)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[doc(alias = "TransactionReceiptEnvelope", alias = "TxReceiptEnvelope")]
pub enum ReceiptEnvelope<T = Log> {
    /// Receipt envelope with no type flag.
    #[cfg_attr(feature = "serde", serde(rename = "0x0", alias = "0x00"))]
    Legacy(ReceiptWithBloom<Receipt<T>>),
    /// Receipt envelope with type flag 1, containing a [EIP-2930] receipt.
    ///
    /// [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    #[cfg_attr(feature = "serde", serde(rename = "0x1", alias = "0x01"))]
    Eip2930(ReceiptWithBloom<Receipt<T>>),
    /// Receipt envelope with type flag 2, containing a [EIP-1559] receipt.
    ///
    /// [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    #[cfg_attr(feature = "serde", serde(rename = "0x2", alias = "0x02"))]
    Eip1559(ReceiptWithBloom<Receipt<T>>),
    /// Receipt envelope with type flag 3, containing a [EIP-4844] receipt.
    ///
    /// [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    #[cfg_attr(feature = "serde", serde(rename = "0x3", alias = "0x03"))]
    Eip4844(ReceiptWithBloom<Receipt<T>>),
    /// Receipt envelope with type flag 4, containing a [EIP-7702] receipt.
    ///
    /// [EIP-7702]: https://eips.ethereum.org/EIPS/eip-7702
    #[cfg_attr(feature = "serde", serde(rename = "0x4", alias = "0x04"))]
    Eip7702(ReceiptWithBloom<Receipt<T>>),
    /// Receipt envelope with type flag 6, containing a [EIP-8141] frame receipt payload.
    ///
    /// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
    #[cfg_attr(feature = "serde", serde(rename = "0x6", alias = "0x06"))]
    Eip8141(FrameReceiptEnvelope<T>),
}

/// An EIP-8141 receipt payload together with the transaction logs flattened across frames.
///
/// The payload is the consensus representation. The flattened log cache is kept separately so
/// that [`TxReceipt::logs`] can return a slice without allocating on every access.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
pub struct FrameReceiptEnvelope<T> {
    /// The consensus EIP-8141 receipt payload.
    payload: FrameReceiptPayload<T>,
    /// Logs in frame execution order.
    logs: Vec<T>,
}

impl<T: Clone> From<FrameReceiptPayload<T>> for FrameReceiptEnvelope<T> {
    fn from(payload: FrameReceiptPayload<T>) -> Self {
        let logs = payload
            .frame_receipts
            .iter()
            .flat_map(|receipt| receipt.logs.iter().cloned())
            .collect();
        Self { payload, logs }
    }
}

impl<T> FrameReceiptEnvelope<T> {
    /// Creates a frame receipt envelope from its consensus payload.
    pub fn new(payload: FrameReceiptPayload<T>) -> Self
    where
        T: Clone,
    {
        payload.into()
    }

    /// Returns the consensus EIP-8141 receipt payload.
    pub const fn payload(&self) -> &FrameReceiptPayload<T> {
        &self.payload
    }

    /// Returns the flattened logs in frame execution order.
    pub fn logs(&self) -> &[T] {
        &self.logs
    }

    /// Splits this envelope into its consensus payload and derived flattened logs.
    pub fn into_parts(self) -> (FrameReceiptPayload<T>, Vec<T>) {
        (self.payload, self.logs)
    }
}

impl<T: Encodable> Encodable for FrameReceiptEnvelope<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.payload.encode(out);
    }

    fn length(&self) -> usize {
        self.payload.length()
    }
}

impl<T: Decodable + Clone> Decodable for FrameReceiptEnvelope<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        FrameReceiptPayload::<T>::decode(buf).map(Into::into)
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for FrameReceiptEnvelope<T>
where
    T: serde::Serialize + AsRef<Log>,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Frame<'a, T> {
            #[serde(with = "alloy_serde::quantity")]
            status: u8,
            #[serde(with = "alloy_serde::quantity")]
            cumulative_gas_used: u64,
            logs: &'a [T],
            logs_bloom: Bloom,
            payer: alloy_primitives::Address,
            frame_receipts: &'a [alloy_eips::eip8141::FrameReceipt<T>],
        }

        Frame {
            status: u8::from(
                self.payload
                    .frame_receipts
                    .iter()
                    .all(|frame| matches!(frame.status, FrameStatus::Success)),
            ),
            cumulative_gas_used: self.payload.cumulative_gas_used,
            logs: &self.logs,
            logs_bloom: logs_bloom(self.logs.iter().map(AsRef::as_ref)),
            payer: self.payload.payer,
            frame_receipts: &self.payload.frame_receipts,
        }
        .serialize(serializer)
    }
}

/// Deserializes a receipt, treating a missing `type` field as [`TxType::Legacy`].
///
/// The `type` field is required by the JSON-RPC specification, but some
/// Ethereum-compatible nodes omit it entirely. A receipt without a type flag is a
/// pre-[EIP-2718] receipt, which is unambiguously legacy, so it is accepted rather
/// than rejected.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + Clone> serde::Deserialize<'de> for ReceiptEnvelope<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ReceiptEnvelopeHelper<T> {
            #[serde(default, rename = "type", with = "alloy_serde::quantity::opt")]
            ty: Option<u8>,
            #[serde(flatten)]
            receipt: ReceiptWithBloom<Receipt<T>>,
            payer: Option<alloy_primitives::Address>,
            frame_receipts: Option<Vec<alloy_eips::eip8141::FrameReceipt<T>>>,
        }

        let ReceiptEnvelopeHelper { ty, receipt, payer, frame_receipts } =
            ReceiptEnvelopeHelper::<T>::deserialize(deserializer)?;
        let ty = ty.unwrap_or(LEGACY_TX_TYPE_ID);
        let ty = TxType::try_from(ty).map_err(serde::de::Error::custom)?;
        if ty == TxType::Eip8141 {
            let payer = payer.ok_or_else(|| serde::de::Error::missing_field("payer"))?;
            let frame_receipts =
                frame_receipts.ok_or_else(|| serde::de::Error::missing_field("frameReceipts"))?;
            let payload = FrameReceiptPayload {
                cumulative_gas_used: receipt.receipt.cumulative_gas_used,
                payer,
                frame_receipts,
            };
            return Ok(Self::Eip8141(payload.into()));
        }

        Self::from_typed(ty, receipt)
            .map_err(|_| serde::de::Error::custom("frame receipt requires a frame payload"))
    }
}

impl<T> ReceiptEnvelope<T> {
    /// Creates the envelope for a given type and receipt.
    ///
    /// Returns the original receipt on error for EIP-8141, which requires a frame payload.
    pub fn from_typed<R>(tx_type: TxType, receipt: R) -> Result<Self, crate::error::ValueError<R>>
    where
        R: Into<ReceiptWithBloom<Receipt<T>>>,
    {
        Ok(match tx_type {
            TxType::Legacy => Self::Legacy(receipt.into()),
            TxType::Eip2930 => Self::Eip2930(receipt.into()),
            TxType::Eip1559 => Self::Eip1559(receipt.into()),
            TxType::Eip4844 => Self::Eip4844(receipt.into()),
            TxType::Eip7702 => Self::Eip7702(receipt.into()),
            TxType::Eip8141 => {
                return Err(crate::error::ValueError::new_static(
                    receipt,
                    "EIP-8141 receipts require a FrameReceiptPayload",
                ));
            }
        })
    }

    /// Converts the receipt's log type by applying a function to each log.
    ///
    /// Returns the receipt with the new log type.
    pub fn map_logs<U: Clone>(self, mut f: impl FnMut(T) -> U) -> ReceiptEnvelope<U> {
        match self {
            Self::Legacy(r) => ReceiptEnvelope::Legacy(r.map_logs(f)),
            Self::Eip2930(r) => ReceiptEnvelope::Eip2930(r.map_logs(f)),
            Self::Eip1559(r) => ReceiptEnvelope::Eip1559(r.map_logs(f)),
            Self::Eip4844(r) => ReceiptEnvelope::Eip4844(r.map_logs(f)),
            Self::Eip7702(r) => ReceiptEnvelope::Eip7702(r.map_logs(f)),
            Self::Eip8141(r) => {
                let (payload, _) = r.into_parts();
                ReceiptEnvelope::Eip8141(payload.map_logs(&mut f).into())
            }
        }
    }

    /// Converts a [`ReceiptEnvelope`] with a custom log type into a [`ReceiptEnvelope`] with the
    /// primitives [`Log`] type by converting the logs.
    ///
    /// This is useful if log types that embed the primitives log type, e.g. the log receipt rpc
    /// type.
    pub fn into_primitives_receipt(self) -> ReceiptEnvelope<Log>
    where
        T: Into<Log>,
    {
        self.map_logs(Into::into)
    }

    /// Return the [`TxType`] of the inner receipt.
    #[doc(alias = "transaction_type")]
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
            Self::Eip4844(_) => TxType::Eip4844,
            Self::Eip7702(_) => TxType::Eip7702,
            Self::Eip8141(_) => TxType::Eip8141,
        }
    }

    /// Return true if the transaction was successful.
    pub fn is_success(&self) -> bool {
        self.status()
    }

    /// Returns the success status of the receipt's transaction.
    pub fn status(&self) -> bool {
        match self.as_receipt() {
            Some(receipt) => receipt.status.coerce_status(),
            None => self.as_eip8141().is_some_and(|receipt| {
                receipt
                    .frame_receipts
                    .iter()
                    .all(|frame| matches!(frame.status, FrameStatus::Success))
            }),
        }
    }

    /// Returns the cumulative gas used at this receipt.
    pub const fn cumulative_gas_used(&self) -> u64 {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.receipt.cumulative_gas_used,
            Self::Eip8141(t) => t.payload.cumulative_gas_used,
        }
    }

    /// Return the receipt logs.
    pub fn logs(&self) -> &[T] {
        match self.as_receipt() {
            Some(receipt) => &receipt.logs,
            None => match self {
                Self::Eip8141(receipt) => &receipt.logs,
                _ => &[],
            },
        }
    }

    /// Consumes the type and returns the logs.
    pub fn into_logs(self) -> Vec<T> {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.receipt.logs,
            Self::Eip8141(t) => t.logs,
        }
    }

    /// Return the receipt's bloom.
    pub fn logs_bloom(&self) -> Bloom
    where
        T: AsRef<Log>,
    {
        match self.as_receipt_with_bloom() {
            Some(receipt) => receipt.logs_bloom,
            None => logs_bloom(self.logs().iter().map(AsRef::as_ref)),
        }
    }

    /// Return the inner receipt with bloom for normal receipt types.
    pub const fn as_receipt_with_bloom(&self) -> Option<&ReceiptWithBloom<Receipt<T>>> {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => Some(t),
            Self::Eip8141(_) => None,
        }
    }

    /// Return the mutable inner receipt with bloom for normal receipt types.
    pub const fn as_receipt_with_bloom_mut(&mut self) -> Option<&mut ReceiptWithBloom<Receipt<T>>> {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => Some(t),
            Self::Eip8141(_) => None,
        }
    }

    /// Consumes the type and returns the underlying [`Receipt`].
    /// Returns the original envelope on error for a frame receipt.
    pub fn into_receipt(self) -> Result<Receipt<T>, crate::error::ValueError<Self>> {
        Ok(match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.receipt,
            Self::Eip8141(_) => {
                return Err(crate::error::ValueError::new_static(
                    self,
                    "EIP-8141 receipts use FrameReceiptPayload",
                ))
            }
        })
    }

    /// Return the inner receipt for normal receipt types.
    pub const fn as_receipt(&self) -> Option<&Receipt<T>> {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => Some(&t.receipt),
            Self::Eip8141(_) => None,
        }
    }

    /// Return the inner EIP-8141 frame receipt payload.
    pub const fn as_eip8141(&self) -> Option<&FrameReceiptPayload<T>> {
        match self {
            Self::Eip8141(t) => Some(&t.payload),
            _ => None,
        }
    }
}

impl<T> TxReceipt for ReceiptEnvelope<T>
where
    T: Clone + fmt::Debug + PartialEq + Eq + Send + Sync + AsRef<Log>,
{
    type Log = T;

    fn status_or_post_state(&self) -> Eip658Value {
        match self.as_receipt() {
            Some(receipt) => receipt.status,
            None => self.status().into(),
        }
    }

    fn status(&self) -> bool {
        Self::status(self)
    }

    /// Return the receipt's bloom.
    fn bloom(&self) -> Bloom {
        match self {
            Self::Legacy(receipt)
            | Self::Eip2930(receipt)
            | Self::Eip1559(receipt)
            | Self::Eip4844(receipt)
            | Self::Eip7702(receipt) => receipt.logs_bloom,
            Self::Eip8141(receipt) => logs_bloom(receipt.logs.iter().map(AsRef::as_ref)),
        }
    }

    fn bloom_cheap(&self) -> Option<Bloom> {
        self.as_receipt_with_bloom().map(|receipt| receipt.logs_bloom)
    }

    /// Returns the cumulative gas used at this receipt.
    fn cumulative_gas_used(&self) -> u64 {
        Self::cumulative_gas_used(self)
    }

    /// Return the receipt logs.
    fn logs(&self) -> &[T] {
        Self::logs(self)
    }

    fn into_logs(self) -> Vec<Self::Log>
    where
        Self::Log: Clone,
    {
        Self::into_logs(self)
    }
}

impl<T: Encodable> ReceiptEnvelope<T> {
    /// Get the length of the inner receipt in the 2718 encoding.
    pub fn inner_length(&self) -> usize {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.length(),
            Self::Eip8141(t) => t.payload.length(),
        }
    }

    /// Calculate the length of the rlp payload of the network encoded receipt.
    pub fn rlp_payload_length(&self) -> usize {
        let length = self.inner_length();
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
        self.network_len()
    }
}

impl Decodable for ReceiptEnvelope {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::network_decode(buf)
            .map_or_else(|_| Err(alloy_rlp::Error::Custom("Unexpected type")), Ok)
    }
}

impl RlpEncodableReceipt for ReceiptEnvelope {
    fn rlp_encoded_length_with_bloom(&self, bloom: &Bloom) -> usize {
        let payload_length = self.eip2718_encoded_length_with_bloom(bloom);
        if self.is_legacy() {
            payload_length
        } else {
            Header { list: false, payload_length }.length() + payload_length
        }
    }

    fn rlp_encode_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        if !self.is_legacy() {
            Header { list: false, payload_length: self.eip2718_encoded_length_with_bloom(bloom) }
                .encode(out);
        }
        self.eip2718_encode_with_bloom(bloom, out);
    }
}

impl RlpDecodableReceipt for ReceiptEnvelope {
    fn rlp_decode_with_bloom(buf: &mut &[u8]) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let receipt = Self::decode(buf)?;
        let logs_bloom = TxReceipt::bloom(&receipt);
        Ok(ReceiptWithBloom { receipt, logs_bloom })
    }
}

impl Eip2718EncodableReceipt for ReceiptEnvelope {
    fn eip2718_encoded_length_with_bloom(&self, bloom: &Bloom) -> usize {
        let type_len = usize::from(!self.is_legacy());
        type_len
            + match self {
                Self::Legacy(receipt)
                | Self::Eip2930(receipt)
                | Self::Eip1559(receipt)
                | Self::Eip4844(receipt)
                | Self::Eip7702(receipt) => receipt.receipt.rlp_encoded_length_with_bloom(bloom),
                Self::Eip8141(receipt) => receipt.payload.length(),
            }
    }

    fn eip2718_encode_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        if !self.is_legacy() {
            out.put_u8(self.ty());
        }
        match self {
            Self::Legacy(receipt)
            | Self::Eip2930(receipt)
            | Self::Eip1559(receipt)
            | Self::Eip4844(receipt)
            | Self::Eip7702(receipt) => receipt.receipt.rlp_encode_with_bloom(bloom, out),
            Self::Eip8141(receipt) => receipt.payload.encode(out),
        }
    }
}

impl Eip2718DecodableReceipt for ReceiptEnvelope {
    fn typed_decode_with_bloom(ty: u8, buf: &mut &[u8]) -> Eip2718Result<ReceiptWithBloom<Self>> {
        let receipt = Self::typed_decode(ty, buf)?;
        let logs_bloom = TxReceipt::bloom(&receipt);
        Ok(ReceiptWithBloom { receipt, logs_bloom })
    }

    fn fallback_decode_with_bloom(buf: &mut &[u8]) -> Eip2718Result<ReceiptWithBloom<Self>> {
        let receipt = Self::fallback_decode(buf)?;
        let logs_bloom = TxReceipt::bloom(&receipt);
        Ok(ReceiptWithBloom { receipt, logs_bloom })
    }
}

impl InMemorySize for ReceiptEnvelope {
    fn size(&self) -> usize {
        core::mem::size_of::<Self>()
            + match self {
                Self::Legacy(receipt)
                | Self::Eip2930(receipt)
                | Self::Eip1559(receipt)
                | Self::Eip4844(receipt)
                | Self::Eip7702(receipt) => {
                    receipt.receipt.logs.iter().map(InMemorySize::size).sum::<usize>()
                }
                Self::Eip8141(receipt) => {
                    receipt.logs.iter().map(InMemorySize::size).sum::<usize>()
                        + receipt
                            .payload
                            .frame_receipts
                            .iter()
                            .map(|frame| {
                                core::mem::size_of_val(frame)
                                    + frame.logs.iter().map(InMemorySize::size).sum::<usize>()
                            })
                            .sum::<usize>()
                }
            }
    }
}

impl Typed2718 for ReceiptEnvelope {
    fn ty(&self) -> u8 {
        match self {
            Self::Legacy(_) => LEGACY_TX_TYPE_ID,
            Self::Eip2930(_) => EIP2930_TX_TYPE_ID,
            Self::Eip1559(_) => EIP1559_TX_TYPE_ID,
            Self::Eip4844(_) => EIP4844_TX_TYPE_ID,
            Self::Eip7702(_) => EIP7702_TX_TYPE_ID,
            Self::Eip8141(_) => FRAME_TX_TYPE,
        }
    }
}

impl IsTyped2718 for ReceiptEnvelope {
    fn is_type(type_id: u8) -> bool {
        <TxType as IsTyped2718>::is_type(type_id)
    }
}

impl Encodable2718 for ReceiptEnvelope {
    fn encode_2718_len(&self) -> usize {
        self.inner_length() + !self.is_legacy() as usize
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        match self.type_flag() {
            None => {}
            Some(ty) => out.put_u8(ty),
        }
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.encode(out),
            Self::Eip8141(t) => t.payload.encode(out),
        }
    }
}

impl Decodable2718 for ReceiptEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        match ty.try_into().map_err(|_| alloy_rlp::Error::Custom("Unexpected type"))? {
            TxType::Eip2930 => Ok(Self::Eip2930(Decodable::decode(buf)?)),
            TxType::Eip1559 => Ok(Self::Eip1559(Decodable::decode(buf)?)),
            TxType::Eip4844 => Ok(Self::Eip4844(Decodable::decode(buf)?)),
            TxType::Eip7702 => Ok(Self::Eip7702(Decodable::decode(buf)?)),
            TxType::Eip8141 => Ok(Self::Eip8141(Decodable::decode(buf)?)),
            TxType::Legacy => Err(Eip2718Error::UnexpectedType(0)),
        }
    }

    fn fallback_decode(buf: &mut &[u8]) -> Eip2718Result<Self> {
        Ok(Self::Legacy(Decodable::decode(buf)?))
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T> arbitrary::Arbitrary<'a> for ReceiptEnvelope<T>
where
    T: arbitrary::Arbitrary<'a> + Clone,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        match u.int_in_range(0..=5)? {
            0 => Ok(Self::Legacy(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            1 => Ok(Self::Eip2930(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            2 => Ok(Self::Eip1559(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            3 => Ok(Self::Eip4844(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            4 => Ok(Self::Eip7702(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            5 => Ok(Self::Eip8141(FrameReceiptPayload::<T>::arbitrary(u)?.into())),
            _ => unreachable!(),
        }
    }
}

/// Bincode-compatible [`ReceiptEnvelope`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) mod serde_bincode_compat {
    use super::FrameReceiptEnvelope;
    use crate::{Receipt, ReceiptWithBloom, TxType};
    use alloc::borrow::Cow;
    use alloy_eips::eip8141::{FrameReceipt, FrameReceiptPayload};
    use alloy_primitives::{Address, Bloom, Log, U8};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::ReceiptEnvelope`] serde implementation.
    ///
    /// Intended to use with the [`serde_with::serde_as`] macro in the following way:
    /// ```rust
    /// use alloy_consensus::{serde_bincode_compat, ReceiptEnvelope};
    /// use serde::{de::DeserializeOwned, Deserialize, Serialize};
    /// use serde_with::serde_as;
    ///
    /// #[serde_as]
    /// #[derive(Serialize, Deserialize)]
    /// struct Data<T: Serialize + DeserializeOwned + Clone + 'static> {
    ///     #[serde_as(as = "serde_bincode_compat::ReceiptEnvelope<'_, T>")]
    ///     receipt: ReceiptEnvelope<T>,
    /// }
    /// ```
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ReceiptEnvelope<'a, T: Clone = Log> {
        #[serde(deserialize_with = "deserde_txtype")]
        tx_type: TxType,
        success: bool,
        cumulative_gas_used: u64,
        logs_bloom: Cow<'a, Bloom>,
        logs: Cow<'a, [T]>,
        payer: Option<Address>,
        frame_receipts: Option<Cow<'a, [FrameReceipt<T>]>>,
    }

    /// Ensures that txtype is deserialized symmetrically as U8
    fn deserde_txtype<'de, D>(deserializer: D) -> Result<TxType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = U8::deserialize(deserializer)?;
        value.to::<u8>().try_into().map_err(serde::de::Error::custom)
    }

    impl<'a, T: Clone> From<&'a super::ReceiptEnvelope<T>> for ReceiptEnvelope<'a, T> {
        fn from(value: &'a super::ReceiptEnvelope<T>) -> Self {
            match value {
                super::ReceiptEnvelope::Eip8141(receipt) => Self {
                    tx_type: value.tx_type(),
                    success: value.status(),
                    cumulative_gas_used: receipt.payload.cumulative_gas_used,
                    logs_bloom: Cow::Owned(Default::default()),
                    logs: Cow::Borrowed(value.logs()),
                    payer: Some(receipt.payload.payer),
                    frame_receipts: Some(Cow::Borrowed(&receipt.payload.frame_receipts)),
                },
                super::ReceiptEnvelope::Legacy(receipt)
                | super::ReceiptEnvelope::Eip2930(receipt)
                | super::ReceiptEnvelope::Eip1559(receipt)
                | super::ReceiptEnvelope::Eip4844(receipt)
                | super::ReceiptEnvelope::Eip7702(receipt) => Self {
                    tx_type: value.tx_type(),
                    success: value.status(),
                    cumulative_gas_used: value.cumulative_gas_used(),
                    logs_bloom: Cow::Borrowed(&receipt.logs_bloom),
                    logs: Cow::Borrowed(value.logs()),
                    payer: None,
                    frame_receipts: None,
                },
            }
        }
    }

    impl<'a, T: Clone> From<ReceiptEnvelope<'a, T>> for super::ReceiptEnvelope<T> {
        fn from(value: ReceiptEnvelope<'a, T>) -> Self {
            let ReceiptEnvelope {
                tx_type,
                success,
                cumulative_gas_used,
                logs_bloom,
                logs,
                payer,
                frame_receipts,
            } = value;
            if tx_type == TxType::Eip8141 {
                return Self::Eip8141(FrameReceiptEnvelope::from(FrameReceiptPayload {
                    cumulative_gas_used,
                    payer: payer.unwrap_or_default(),
                    frame_receipts: frame_receipts.map(Cow::into_owned).unwrap_or_default(),
                }));
            }
            let receipt = ReceiptWithBloom {
                receipt: Receipt {
                    status: success.into(),
                    cumulative_gas_used,
                    logs: logs.into_owned(),
                },
                logs_bloom: logs_bloom.into_owned(),
            };
            match tx_type {
                TxType::Legacy => Self::Legacy(receipt),
                TxType::Eip2930 => Self::Eip2930(receipt),
                TxType::Eip1559 => Self::Eip1559(receipt),
                TxType::Eip4844 => Self::Eip4844(receipt),
                TxType::Eip7702 => Self::Eip7702(receipt),
                TxType::Eip8141 => unreachable!("handled above"),
            }
        }
    }

    impl<T: Serialize + Clone> SerializeAs<super::ReceiptEnvelope<T>> for ReceiptEnvelope<'_, T> {
        fn serialize_as<S>(
            source: &super::ReceiptEnvelope<T>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            ReceiptEnvelope::<'_, T>::from(source).serialize(serializer)
        }
    }

    impl<'de, T: Deserialize<'de> + Clone> DeserializeAs<'de, super::ReceiptEnvelope<T>>
        for ReceiptEnvelope<'de, T>
    {
        fn deserialize_as<D>(deserializer: D) -> Result<super::ReceiptEnvelope<T>, D::Error>
        where
            D: Deserializer<'de>,
        {
            ReceiptEnvelope::<'_, T>::deserialize(deserializer).map(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::{serde_bincode_compat, ReceiptEnvelope};
        use alloy_primitives::Log;
        use arbitrary::Arbitrary;
        use bincode::config;
        use rand::Rng;
        use serde::{Deserialize, Serialize};
        use serde_with::serde_as;

        #[test]
        fn test_receipt_envelope_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data {
                #[serde_as(as = "serde_bincode_compat::ReceiptEnvelope<'_>")]
                transaction: ReceiptEnvelope<Log>,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let mut data = Data {
                transaction: ReceiptEnvelope::arbitrary(&mut arbitrary::Unstructured::new(&bytes))
                    .unwrap(),
            };

            // ensure we have proper roundtrip data
            if let Some(receipt) = data.transaction.as_receipt_with_bloom_mut() {
                receipt.receipt.status = true.into();
            }

            let encoded = bincode::serde::encode_to_vec(&data, config::legacy()).unwrap();
            let (decoded, _) =
                bincode::serde::decode_from_slice::<Data, _>(&encoded, config::legacy()).unwrap();
            assert_eq!(decoded, data);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        Receipt, ReceiptEnvelope, RlpDecodableReceipt, RlpEncodableReceipt, TxReceipt, TxType,
    };
    use alloy_eips::{
        eip2718::{Decodable2718, Encodable2718},
        eip8141::{
            constants::FRAME_TX_TYPE, FrameGasUsed, FrameReceipt, FrameReceiptPayload, FrameStatus,
        },
    };
    use alloy_primitives::{Address, Bloom, Log};

    #[cfg(feature = "serde")]
    #[test]
    fn deser_pre658_receipt_envelope() {
        use crate::Receipt;
        use alloy_primitives::b256;

        let receipt = super::ReceiptWithBloom::<Receipt<()>> {
            receipt: super::Receipt {
                status: super::Eip658Value::PostState(b256!(
                    "284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10"
                )),
                cumulative_gas_used: 0,
                logs: Default::default(),
            },
            logs_bloom: Default::default(),
        };

        let json = serde_json::to_string(&receipt).unwrap();

        println!("Serialized {json}");

        let receipt: super::ReceiptWithBloom<Receipt<()>> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            receipt.receipt.status,
            super::Eip658Value::PostState(b256!(
                "284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10"
            ))
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deser_receipt_envelope_without_type() {
        let inner = super::ReceiptWithBloom::<Receipt<()>> {
            receipt: Receipt {
                status: super::Eip658Value::Eip658(true),
                cumulative_gas_used: 0xc3b68,
                logs: Default::default(),
            },
            logs_bloom: Default::default(),
        };
        let mut json = serde_json::to_value(&inner).unwrap();
        assert!(json.get("type").is_none());

        let envelope: ReceiptEnvelope<()> = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(envelope, ReceiptEnvelope::Legacy(inner.clone()));

        // An explicit type flag is still honored.
        json["type"] = "0x2".into();
        let envelope: ReceiptEnvelope<()> = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(envelope, ReceiptEnvelope::Eip1559(inner));

        // An unknown type flag is still rejected.
        json["type"] = "0x7f".into();
        serde_json::from_value::<ReceiptEnvelope<()>>(json).unwrap_err();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn standard_receipt_json_shape_is_preserved() {
        let inner = super::ReceiptWithBloom::<Receipt<Log>> {
            receipt: Receipt {
                status: super::Eip658Value::Eip658(true),
                cumulative_gas_used: 21_000,
                logs: Default::default(),
            },
            logs_bloom: Default::default(),
        };
        let envelope = ReceiptEnvelope::Eip1559(inner);

        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["type"], "0x2");
        assert_eq!(json["status"], "0x1");
        assert_eq!(json["cumulativeGasUsed"], "0x5208");

        let decoded: ReceiptEnvelope<Log> = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn eip8141_receipt_json_roundtrip() {
        let envelope = ReceiptEnvelope::Eip8141(
            FrameReceiptPayload {
                cumulative_gas_used: 42,
                payer: Address::repeat_byte(0x11),
                frame_receipts: alloc::vec![FrameReceipt {
                    status: FrameStatus::Success,
                    gas_used: FrameGasUsed { execution: 21, state: 1 },
                    logs: alloc::vec![Log::default()],
                }],
            }
            .into(),
        );

        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["type"], "0x6");
        assert_eq!(json["status"], "0x1");
        assert_eq!(json["cumulativeGasUsed"], "0x2a");
        assert!(json.get("payload").is_none());

        let decoded: ReceiptEnvelope<Log> = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn convert_envelope() {
        let receipt = Receipt::<Log>::default();
        let envelope = ReceiptEnvelope::from_typed(TxType::Eip7702, receipt).unwrap();
        assert!(matches!(envelope, ReceiptEnvelope::Eip7702(_)));
    }

    #[test]
    fn eip8141_receipt_roundtrip_uses_frame_payload() {
        let envelope = ReceiptEnvelope::Eip8141(
            FrameReceiptPayload {
                cumulative_gas_used: 42,
                payer: Address::repeat_byte(0x11),
                frame_receipts: alloc::vec![FrameReceipt {
                    status: FrameStatus::Success,
                    gas_used: FrameGasUsed { execution: 21, state: 0 },
                    logs: alloc::vec![Log::default()],
                }],
            }
            .into(),
        );

        let mut encoded = Vec::new();
        envelope.encode_2718(&mut encoded);
        assert_eq!(encoded[0], FRAME_TX_TYPE);

        let mut payload = encoded[1..].as_ref();
        let decoded = ReceiptEnvelope::typed_decode(FRAME_TX_TYPE, &mut payload).unwrap();
        assert_eq!(decoded, envelope);
        assert!(decoded.as_receipt_with_bloom().is_none());
        assert_eq!(decoded.as_eip8141().unwrap().frame_receipts.len(), 1);

        let logs_bloom = TxReceipt::bloom(&decoded);
        assert_ne!(logs_bloom, Bloom::default());

        let mut network_encoded = Vec::new();
        decoded.rlp_encode_with_bloom(&logs_bloom, &mut network_encoded);
        assert_eq!(network_encoded.len(), decoded.rlp_encoded_length_with_bloom(&logs_bloom));
        let decoded_with_bloom =
            ReceiptEnvelope::rlp_decode_with_bloom(&mut network_encoded.as_slice()).unwrap();
        assert_eq!(decoded_with_bloom.receipt, decoded);
        assert_eq!(decoded_with_bloom.logs_bloom, logs_bloom);
    }

    #[test]
    fn frame_receipt_log_mapping_keeps_payload_and_flattened_logs_in_sync() {
        let envelope = ReceiptEnvelope::Eip8141(
            FrameReceiptPayload {
                cumulative_gas_used: 1,
                payer: Address::ZERO,
                frame_receipts: alloc::vec![FrameReceipt {
                    status: FrameStatus::Success,
                    gas_used: FrameGasUsed::default(),
                    logs: alloc::vec![10u64, 20],
                }],
            }
            .into(),
        );

        let mut calls = 0;
        let mapped = envelope.map_logs(|_| {
            calls += 1;
            calls
        });
        let ReceiptEnvelope::Eip8141(mapped) = mapped else { unreachable!() };

        assert_eq!(calls, 2);
        assert_eq!(mapped.logs(), &[1, 2]);
        assert_eq!(mapped.payload().frame_receipts[0].logs, [1, 2]);
    }
}
