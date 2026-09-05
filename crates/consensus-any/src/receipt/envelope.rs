use alloc::vec::Vec;
use alloy_consensus::{Eip658Value, Receipt, ReceiptEnvelope, ReceiptWithBloom, TxReceipt, TxType};
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Result, Encodable2718},
    Typed2718,
};
use alloy_primitives::{bytes::BufMut, Bloom, Log};
use alloy_rlp::{Decodable, Encodable};
use core::fmt;

/// A lossless Ethereum receipt, or a conventional receipt from another network.
/// Unknown transaction types retain their type byte and conventional receipt payload.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(untagged, bound(serialize = "T: serde::Serialize + AsRef<Log>"))
)]
pub enum AnyReceiptEnvelope<T = Log> {
    /// A known Ethereum receipt, including EIP-8141 frame receipts.
    Ethereum(ReceiptEnvelope<T>),
    /// A network-specific conventional receipt.
    Other {
        /// Receipt fields and bloom.
        #[cfg_attr(feature = "serde", serde(flatten))]
        inner: ReceiptWithBloom<Receipt<T>>,
        /// Transaction type.
        #[cfg_attr(feature = "serde", serde(rename = "type", with = "alloy_serde::quantity"))]
        r#type: u8,
    },
}

impl<T> From<ReceiptEnvelope<T>> for AnyReceiptEnvelope<T> {
    fn from(receipt: ReceiptEnvelope<T>) -> Self {
        Self::Ethereum(receipt)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + Clone> serde::Deserialize<'de> for AnyReceiptEnvelope<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields<T> {
            #[serde(rename = "type", with = "alloy_serde::quantity")]
            ty: u8,
            #[serde(flatten)]
            receipt: ReceiptWithBloom<Receipt<T>>,
            payer: Option<alloy_primitives::Address>,
            frame_receipts: Option<Vec<alloy_eips::eip8141::FrameReceipt<T>>>,
        }
        let fields = Fields::<T>::deserialize(deserializer)?;
        if fields.ty == 6 {
            let payload = alloy_eips::eip8141::FrameReceiptPayload {
                cumulative_gas_used: fields.receipt.receipt.cumulative_gas_used,
                payer: fields.payer.ok_or_else(|| serde::de::Error::missing_field("payer"))?,
                frame_receipts: fields
                    .frame_receipts
                    .ok_or_else(|| serde::de::Error::missing_field("frameReceipts"))?,
            };
            return Ok(Self::Ethereum(ReceiptEnvelope::Eip8141(payload.into())));
        }
        match TxType::try_from(fields.ty) {
            Ok(ty) => ReceiptEnvelope::from_typed(ty, fields.receipt)
                .map(Self::Ethereum)
                .map_err(|_| serde::de::Error::custom("frame receipt requires a frame payload")),
            Err(_) => Ok(Self::Other { inner: fields.receipt, r#type: fields.ty }),
        }
    }
}

impl<T> AnyReceiptEnvelope<T> {
    /// Returns the transaction type.
    pub const fn tx_type(&self) -> u8 {
        match self {
            Self::Ethereum(receipt) => receipt.tx_type() as u8,
            Self::Other { r#type, .. } => *r#type,
        }
    }

    /// Returns whether this is a legacy receipt.
    pub const fn is_legacy(&self) -> bool {
        self.tx_type() == 0
    }

    /// Returns the derived transaction status.
    pub fn is_success(&self) -> bool {
        self.status()
    }

    /// Returns the transaction status; frame receipts succeed when every frame succeeds.
    pub fn status(&self) -> bool {
        match self {
            Self::Ethereum(receipt) => receipt.status(),
            Self::Other { inner, .. } => inner.receipt.status.coerce_status(),
        }
    }

    /// Returns the bloom, computing it for frame receipts.
    pub fn bloom(&self) -> Bloom
    where
        T: AsRef<Log>,
    {
        match self {
            Self::Ethereum(receipt) => receipt.logs_bloom(),
            Self::Other { inner, .. } => inner.logs_bloom,
        }
    }

    /// Returns a cached bloom when the receipt carries one.
    pub const fn bloom_ref(&self) -> Option<&Bloom> {
        match self {
            Self::Ethereum(receipt) => match receipt.as_receipt_with_bloom() {
                Some(receipt) => Some(&receipt.logs_bloom),
                None => None,
            },
            Self::Other { inner, .. } => Some(&inner.logs_bloom),
        }
    }

    /// Returns cumulative gas used.
    pub const fn cumulative_gas_used(&self) -> u64 {
        match self {
            Self::Ethereum(receipt) => receipt.cumulative_gas_used(),
            Self::Other { inner, .. } => inner.receipt.cumulative_gas_used,
        }
    }

    /// Returns the receipt logs.
    pub fn logs(&self) -> &[T] {
        match self {
            Self::Ethereum(receipt) => receipt.logs(),
            Self::Other { inner, .. } => &inner.receipt.logs,
        }
    }
}

impl<T> TxReceipt for AnyReceiptEnvelope<T>
where
    T: Clone + fmt::Debug + PartialEq + Eq + Send + Sync + AsRef<Log>,
{
    type Log = T;
    fn status_or_post_state(&self) -> Eip658Value {
        match self {
            Self::Ethereum(receipt) => receipt.status_or_post_state(),
            Self::Other { inner, .. } => inner.receipt.status,
        }
    }
    fn status(&self) -> bool {
        Self::status(self)
    }
    fn bloom(&self) -> Bloom {
        Self::bloom(self)
    }
    fn bloom_cheap(&self) -> Option<Bloom> {
        self.bloom_ref().copied()
    }
    fn cumulative_gas_used(&self) -> u64 {
        Self::cumulative_gas_used(self)
    }
    fn logs(&self) -> &[T] {
        Self::logs(self)
    }
    fn into_logs(self) -> Vec<T> {
        match self {
            Self::Ethereum(receipt) => receipt.into_logs(),
            Self::Other { inner, .. } => inner.receipt.logs,
        }
    }
}

impl Typed2718 for AnyReceiptEnvelope {
    fn ty(&self) -> u8 {
        self.tx_type()
    }
}

impl<T: Encodable> AnyReceiptEnvelope<T> {
    /// Returns the RLP payload length of the network encoding.
    pub fn rlp_payload_length(&self) -> usize {
        match self {
            Self::Ethereum(receipt) => receipt.rlp_payload_length(),
            Self::Other { inner, .. } => inner.length() + usize::from(!self.is_legacy()),
        }
    }
}

impl Encodable2718 for AnyReceiptEnvelope {
    fn encode_2718_len(&self) -> usize {
        match self {
            Self::Ethereum(receipt) => receipt.encode_2718_len(),
            Self::Other { inner, .. } => inner.length() + usize::from(!self.is_legacy()),
        }
    }
    fn encode_2718(&self, out: &mut dyn BufMut) {
        match self {
            Self::Ethereum(receipt) => receipt.encode_2718(out),
            Self::Other { inner, .. } => {
                if let Some(ty) = self.type_flag() {
                    out.put_u8(ty);
                }
                inner.encode(out);
            }
        }
    }
}

impl Decodable2718 for AnyReceiptEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        match TxType::try_from(ty) {
            Ok(TxType::Legacy) => ReceiptEnvelope::fallback_decode(buf).map(Self::Ethereum),
            Ok(_) => ReceiptEnvelope::typed_decode(ty, buf).map(Self::Ethereum),
            Err(_) => Ok(Self::Other { inner: Decodable::decode(buf)?, r#type: ty }),
        }
    }
    fn fallback_decode(buf: &mut &[u8]) -> Eip2718Result<Self> {
        ReceiptEnvelope::fallback_decode(buf).map(Self::Ethereum)
    }
}
