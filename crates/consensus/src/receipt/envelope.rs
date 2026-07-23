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
    eip8141::{constants::FRAME_TX_TYPE, FrameReceipt, FrameReceiptPayload},
    Typed2718,
};
use alloy_primitives::{logs_bloom, Bloom, Log};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};
use core::fmt;

/// Receipt envelope, as defined in [EIP-2718].
///
/// This enum distinguishes between tagged and untagged legacy receipts, as the
/// in-protocol Merkle tree may commit to EITHER 0-prefixed or raw. Therefore
/// we must ensure that encoding returns the precise byte-array that was
/// decoded, preserving the presence or absence of the `TransactionType` flag.
///
/// Transaction receipt payloads are specified in their respective EIPs.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
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
    Eip8141(FrameReceiptPayload<T>),
}

impl<T> ReceiptEnvelope<T> {
    /// Creates the envelope for a given type and receipt.
    pub fn from_typed<R>(tx_type: TxType, receipt: R) -> Self
    where
        R: Into<ReceiptWithBloom<Receipt<T>>>,
    {
        match tx_type {
            TxType::Legacy => Self::Legacy(receipt.into()),
            TxType::Eip2930 => Self::Eip2930(receipt.into()),
            TxType::Eip1559 => Self::Eip1559(receipt.into()),
            TxType::Eip4844 => Self::Eip4844(receipt.into()),
            TxType::Eip7702 => Self::Eip7702(receipt.into()),
            TxType::Eip8141 => {
                panic!(
                    "EIP-8141 receipts use FrameReceiptPayload; construct ReceiptEnvelope::Eip8141"
                )
            }
        }
    }

    /// Converts the receipt's log type by applying a function to each log.
    ///
    /// Returns the receipt with the new log type.
    pub fn map_logs<U>(self, mut f: impl FnMut(T) -> U) -> ReceiptEnvelope<U> {
        match self {
            Self::Legacy(r) => ReceiptEnvelope::Legacy(r.map_logs(f)),
            Self::Eip2930(r) => ReceiptEnvelope::Eip2930(r.map_logs(f)),
            Self::Eip1559(r) => ReceiptEnvelope::Eip1559(r.map_logs(f)),
            Self::Eip4844(r) => ReceiptEnvelope::Eip4844(r.map_logs(f)),
            Self::Eip7702(r) => ReceiptEnvelope::Eip7702(r.map_logs(f)),
            Self::Eip8141(r) => ReceiptEnvelope::Eip8141(FrameReceiptPayload {
                cumulative_gas_used: r.cumulative_gas_used,
                payer: r.payer,
                frame_receipts: r
                    .frame_receipts
                    .into_iter()
                    .map(|receipt| FrameReceipt {
                        status: receipt.status,
                        gas_used: receipt.gas_used,
                        logs: receipt.logs.into_iter().map(&mut f).collect(),
                    })
                    .collect(),
            }),
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
    pub const fn is_success(&self) -> bool {
        self.status()
    }

    /// Returns the success status of the receipt's transaction.
    pub const fn status(&self) -> bool {
        match self.as_receipt() {
            Some(receipt) => receipt.status.coerce_status(),
            None => true,
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
            Self::Eip8141(t) => t.cumulative_gas_used,
        }
    }

    /// Return the receipt logs.
    pub fn logs(&self) -> &[T] {
        match self.as_receipt() {
            Some(receipt) => &receipt.logs,
            None => &[],
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
            Self::Eip8141(t) => {
                t.frame_receipts.into_iter().flat_map(|receipt| receipt.logs).collect()
            }
        }
    }

    /// Return the receipt's bloom.
    pub const fn logs_bloom(&self) -> &Bloom {
        match self.as_receipt_with_bloom() {
            Some(receipt) => &receipt.logs_bloom,
            None => panic!("EIP-8141 receipts do not carry a top-level logs bloom"),
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
    pub fn into_receipt(self) -> Receipt<T> {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.receipt,
            Self::Eip8141(_) => panic!("EIP-8141 receipts use FrameReceiptPayload"),
        }
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
            Self::Eip8141(t) => Some(t),
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
            None => Eip658Value::success(),
        }
    }

    fn status(&self) -> bool {
        ReceiptEnvelope::status(self)
    }

    /// Return the receipt's bloom.
    fn bloom(&self) -> Bloom {
        match self {
            Self::Legacy(receipt)
            | Self::Eip2930(receipt)
            | Self::Eip1559(receipt)
            | Self::Eip4844(receipt)
            | Self::Eip7702(receipt) => receipt.logs_bloom,
            Self::Eip8141(receipt) => logs_bloom(
                receipt
                    .frame_receipts
                    .iter()
                    .flat_map(|frame| frame.logs.iter().map(AsRef::as_ref)),
            ),
        }
    }

    fn bloom_cheap(&self) -> Option<Bloom> {
        self.as_receipt_with_bloom().map(|receipt| receipt.logs_bloom)
    }

    /// Returns the cumulative gas used at this receipt.
    fn cumulative_gas_used(&self) -> u64 {
        ReceiptEnvelope::cumulative_gas_used(self)
    }

    /// Return the receipt logs.
    fn logs(&self) -> &[T] {
        ReceiptEnvelope::logs(self)
    }

    fn into_logs(self) -> Vec<Self::Log>
    where
        Self::Log: Clone,
    {
        ReceiptEnvelope::into_logs(self)
    }
}

impl ReceiptEnvelope {
    /// Get the length of the inner receipt in the 2718 encoding.
    pub fn inner_length(&self) -> usize {
        match self {
            Self::Legacy(t)
            | Self::Eip2930(t)
            | Self::Eip1559(t)
            | Self::Eip4844(t)
            | Self::Eip7702(t) => t.length(),
            Self::Eip8141(t) => t.length(),
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
                Self::Eip8141(receipt) => receipt.length(),
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
            Self::Eip8141(receipt) => receipt.encode(out),
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
                Self::Eip8141(receipt) => receipt
                    .frame_receipts
                    .iter()
                    .map(|frame| {
                        core::mem::size_of_val(frame)
                            + frame.logs.iter().map(InMemorySize::size).sum::<usize>()
                    })
                    .sum::<usize>(),
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
            Self::Eip8141(t) => t.encode(out),
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
    T: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        match u.int_in_range(0..=5)? {
            0 => Ok(Self::Legacy(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            1 => Ok(Self::Eip2930(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            2 => Ok(Self::Eip1559(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            3 => Ok(Self::Eip4844(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            4 => Ok(Self::Eip7702(ReceiptWithBloom::<Receipt<T>>::arbitrary(u)?)),
            5 => Ok(Self::Eip8141(FrameReceiptPayload::<T>::arbitrary(u)?)),
            _ => unreachable!(),
        }
    }
}

/// Bincode-compatible [`ReceiptEnvelope`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) mod serde_bincode_compat {
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
                super::ReceiptEnvelope::Eip8141(payload) => Self {
                    tx_type: value.tx_type(),
                    success: true,
                    cumulative_gas_used: payload.cumulative_gas_used,
                    logs_bloom: Cow::Owned(Default::default()),
                    logs: Cow::Owned(value.clone().into_logs()),
                    payer: Some(payload.payer),
                    frame_receipts: Some(Cow::Borrowed(&payload.frame_receipts)),
                },
                _ => Self {
                    tx_type: value.tx_type(),
                    success: value.status(),
                    cumulative_gas_used: value.cumulative_gas_used(),
                    logs_bloom: Cow::Borrowed(value.logs_bloom()),
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
                return Self::Eip8141(FrameReceiptPayload {
                    cumulative_gas_used,
                    payer: payer.unwrap_or_default(),
                    frame_receipts: frame_receipts.map(Cow::into_owned).unwrap_or_default(),
                });
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
            data.transaction.as_receipt_with_bloom_mut().unwrap().receipt.status = true.into();

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
        eip8141::{constants::FRAME_TX_TYPE, FrameReceipt, FrameReceiptPayload, FrameStatus},
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

    #[test]
    fn convert_envelope() {
        let receipt = Receipt::<Log>::default();
        let _envelope = ReceiptEnvelope::from_typed(TxType::Eip7702, receipt);
    }

    #[test]
    fn eip8141_receipt_roundtrip_uses_frame_payload() {
        let envelope = ReceiptEnvelope::Eip8141(FrameReceiptPayload {
            cumulative_gas_used: 42,
            payer: Address::repeat_byte(0x11),
            frame_receipts: alloc::vec![FrameReceipt {
                status: FrameStatus::Success,
                gas_used: 21,
                logs: alloc::vec![Log::default()],
            }],
        });

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
}
