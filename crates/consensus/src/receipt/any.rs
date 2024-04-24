use crate::{ReceiptWithBloom, TxReceipt};
use alloy_eips::eip2718::{Decodable2718, Encodable2718};
use alloy_primitives::{bytes::BufMut, Bloom, Log};
use alloy_rlp::{Decodable, Encodable};

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
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnyReceiptEnvelope<T = Log> {
    /// The receipt envelope.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: ReceiptWithBloom<T>,
    /// The transaction type.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::num::u8_via_ruint"))]
    pub r#type: u8,
}

impl<T> AnyReceiptEnvelope<T>
where
    T: Encodable,
{
    /// Calculate the length of the rlp payload of the network encoded receipt.
    pub fn rlp_payload_length(&self) -> usize {
        let length = self.inner.length();
        if self.is_legacy() {
            length
        } else {
            length + 1
        }
    }
}

impl<T> AnyReceiptEnvelope<T> {
    /// Returns whether this is a legacy receipt (type 0)
    pub const fn is_legacy(&self) -> bool {
        self.r#type == 0
    }

    /// Return true if the transaction was successful.
    pub const fn is_success(&self) -> bool {
        self.status()
    }

    /// Returns the success status of the receipt's transaction.
    pub const fn status(&self) -> bool {
        self.inner.receipt.status
    }

    /// Return the receipt's bloom.
    pub const fn bloom(&self) -> Bloom {
        self.inner.logs_bloom
    }

    /// Returns the cumulative gas used at this receipt.
    pub const fn cumulative_gas_used(&self) -> u128 {
        self.inner.receipt.cumulative_gas_used
    }

    /// Return the receipt logs.
    pub fn logs(&self) -> &[T] {
        &self.inner.receipt.logs
    }
}

impl<T> TxReceipt<T> for AnyReceiptEnvelope<T> {
    /// Returns the success status of the receipt's transaction.
    fn status(&self) -> bool {
        self.inner.receipt.status
    }

    /// Return the receipt's bloom.
    fn bloom(&self) -> Bloom {
        self.inner.logs_bloom
    }

    /// Returns the cumulative gas used at this receipt.
    fn cumulative_gas_used(&self) -> u128 {
        self.inner.receipt.cumulative_gas_used
    }

    /// Return the receipt logs.
    fn logs(&self) -> &[T] {
        &self.inner.receipt.logs
    }
}

impl Encodable2718 for AnyReceiptEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self.r#type {
            0 => None,
            ty => Some(ty),
        }
    }

    fn encode_2718_len(&self) -> usize {
        self.inner.length() + !self.is_legacy() as usize
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        match self.type_flag() {
            None => {}
            Some(ty) => out.put_u8(ty),
        }
        self.inner.encode(out);
    }
}

impl Decodable2718 for AnyReceiptEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let receipt = Decodable::decode(buf)?;
        Ok(Self { inner: receipt, r#type: ty })
    }

    fn fallback_decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::typed_decode(0, buf)
    }
}
