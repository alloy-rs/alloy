use alloc::vec::Vec;
use alloy_consensus::{Eip658Value, Receipt, ReceiptWithBloom, TxReceipt};
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Error, Eip2718Result, Encodable2718},
    Typed2718,
};
use alloy_primitives::{bytes::BufMut, Bloom, Log};
use alloy_rlp::{Decodable, Encodable};
use core::fmt;

/// Receipt envelope, as defined in [EIP-2718].
///
/// Represents untagged legacy receipts and typed EIP-2718 variants. Binary decoding rejects a
/// literal `0x00` type prefix; type ID 0 uses the untagged legacy encoding.
///
/// Transaction receipt payloads are specified in their respective EIPs.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[doc(alias = "AnyTransactionReceiptEnvelope", alias = "AnyTxReceiptEnvelope")]
pub struct AnyReceiptEnvelope<T = Log> {
    /// The receipt envelope.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: ReceiptWithBloom<Receipt<T>>,
    /// The transaction type.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub r#type: u8,
}

impl<T> AnyReceiptEnvelope<T> {
    /// Returns whether this is a legacy receipt (type 0)
    pub const fn is_legacy(&self) -> bool {
        self.r#type == 0
    }
}

/// Deserializes in a single pass over the object. Flattening the receipt would buffer every
/// field of the object, twice for the fields of the receipt itself.
#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for AnyReceiptEnvelope<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct TypeValue(#[serde(with = "alloy_serde::quantity")] u8);

        let (r#type, inner) = alloy_consensus::private::deserialize_receipt_with_type(
            deserializer,
            |ty: Option<TypeValue>| ty.map(|TypeValue(ty)| ty).ok_or("missing field `type`"),
        )?;
        Ok(Self { inner, r#type })
    }
}

impl<T: Encodable> AnyReceiptEnvelope<T> {
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
    /// Return true if the transaction was successful.
    ///
    /// ## Note
    ///
    /// This method may not accurately reflect the status of the transaction
    /// for transactions before [EIP-658].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    pub const fn is_success(&self) -> bool {
        self.status()
    }

    /// Returns the success status of the receipt's transaction.
    ///
    /// ## Note
    ///
    /// This method may not accurately reflect the status of the transaction
    /// for transactions before [EIP-658].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    pub const fn status(&self) -> bool {
        self.inner.receipt.status.coerce_status()
    }

    /// Return the receipt's bloom.
    pub const fn bloom(&self) -> Bloom {
        self.inner.logs_bloom
    }

    /// Return a reference to the receipt's bloom.
    pub const fn bloom_ref(&self) -> &Bloom {
        &self.inner.logs_bloom
    }

    /// Returns the cumulative gas used at this receipt.
    pub const fn cumulative_gas_used(&self) -> u64 {
        self.inner.receipt.cumulative_gas_used
    }

    /// Return the receipt logs.
    pub fn logs(&self) -> &[T] {
        &self.inner.receipt.logs
    }
}

impl<T> TxReceipt for AnyReceiptEnvelope<T>
where
    T: Clone + fmt::Debug + PartialEq + Eq + Send + Sync,
{
    type Log = T;

    fn status_or_post_state(&self) -> Eip658Value {
        self.inner.receipt.status
    }

    fn status(&self) -> bool {
        self.status()
    }

    fn bloom(&self) -> Bloom {
        self.bloom()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.cumulative_gas_used()
    }

    fn logs(&self) -> &[T] {
        Self::logs(self)
    }

    fn into_logs(self) -> Vec<Self::Log> {
        self.inner.receipt.logs
    }
}

impl Typed2718 for AnyReceiptEnvelope {
    fn ty(&self) -> u8 {
        self.r#type
    }
}

impl Encodable2718 for AnyReceiptEnvelope {
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
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        // Legacy receipts are untagged: `encode_2718` never emits a `0x00` type byte, so a
        // literal `0x00` prefix must be rejected rather than decoded as legacy, which would not
        // round-trip. Untagged legacy receipts are handled by `fallback_decode`.
        if ty == 0 {
            return Err(Eip2718Error::UnexpectedType(ty));
        }
        let receipt = Decodable::decode(buf)?;
        Ok(Self { inner: receipt, r#type: ty })
    }

    fn fallback_decode(buf: &mut &[u8]) -> Eip2718Result<Self> {
        let receipt = Decodable::decode(buf)?;
        Ok(Self { inner: receipt, r#type: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloy_rlp::Header;

    #[cfg(feature = "serde")]
    use alloy_primitives::B256;

    fn envelope(ty: u8) -> AnyReceiptEnvelope {
        AnyReceiptEnvelope { inner: Default::default(), r#type: ty }
    }

    /// Wraps an EIP-2718 payload in the RLP string header used by the network encoding of typed
    /// receipts.
    fn network_framed(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        Header { list: false, payload_length: payload.len() }.encode(&mut out);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn legacy_receipt_roundtrips_untagged() {
        let envelope = envelope(0);
        let encoded = envelope.encoded_2718();
        assert!(encoded[0] >= 0xc0, "sanity: legacy receipts are encoded as a bare RLP list");
        // The network encoding of a legacy receipt is the bare list as well, without a header.
        let mut network = Vec::new();
        envelope.network_encode(&mut network);
        assert_eq!(network, encoded);

        assert_eq!(AnyReceiptEnvelope::decode_2718_exact(&encoded).unwrap(), envelope);
        assert_eq!(AnyReceiptEnvelope::network_decode(&mut encoded.as_slice()).unwrap(), envelope);
    }

    #[test]
    fn typed_receipt_roundtrips() {
        // A non-Ethereum type id, since this envelope is the catch-all for unknown networks.
        let envelope = envelope(0x7e);
        let encoded = envelope.encoded_2718();
        assert_eq!(encoded[0], 0x7e, "sanity: typed receipts are prefixed with their type byte");
        let mut network = Vec::new();
        envelope.network_encode(&mut network);
        assert_eq!(network, network_framed(&encoded));

        assert_eq!(AnyReceiptEnvelope::decode_2718_exact(&encoded).unwrap(), envelope);
        assert_eq!(AnyReceiptEnvelope::network_decode(&mut network.as_slice()).unwrap(), envelope);
    }

    #[test]
    fn tagged_legacy_receipt_is_rejected() {
        // Per EIP-2718, legacy receipts are untagged and `0x00` is not an assigned type. The
        // encoder never emits a `0x00` prefix, so accepting one on decode would re-encode the
        // receipt differently from the bytes that were decoded.
        let mut tagged = vec![0x00];
        tagged.extend_from_slice(&envelope(0).encoded_2718());
        let tagged_network = network_framed(&tagged);

        assert!(matches!(
            AnyReceiptEnvelope::decode_2718_exact(&tagged),
            Err(Eip2718Error::UnexpectedType(0))
        ));
        assert!(matches!(
            AnyReceiptEnvelope::network_decode(&mut tagged_network.as_slice()),
            Err(Eip2718Error::UnexpectedType(0))
        ));
    }

    /// Compares the envelope with the `#[serde(flatten)]` derive it used before, as text and as a
    /// `Value`, alone and flattened.
    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_matches_flattened_derive() {
        #[derive(serde::Deserialize)]
        struct Flattened {
            #[serde(flatten)]
            inner: ReceiptWithBloom<Receipt<Log>>,
            #[serde(rename = "type", with = "alloy_serde::quantity")]
            ty: u8,
        }

        #[derive(serde::Deserialize)]
        struct Outer<R> {
            #[serde(flatten)]
            receipt: R,
            #[serde(rename = "blockHash")]
            _block_hash: Option<B256>,
        }

        const ROOT: &str = "0x284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10";
        const LOG: &str =
            r#"{"address":"0x0000000000000000000000000000000000000001","topics":[],"data":"0x01"}"#;

        // Fields added to a receipt with `cumulativeGasUsed`, `logs` and `logsBloom`.
        let cases = [
            r#""type":"0x0","status":"0x1""#,
            r#""type":"0x2","status":"0x0""#,
            r#""type":"0x7e","root":"ROOT""#,
            r#""type":"0xff","status":"0x1","root":"ROOT""#,
            r#""type":2,"status":"0x1""#,
            // `type` is required
            r#""status":"0x1""#,
            r#""type":null,"status":"0x1""#,
            r#""type":"0x100","status":"0x1""#,
            r#""type":true,"status":"0x1""#,
            // status vs root
            r#""type":"0x1""#,
            r#""type":"0x1","status":"0x2""#,
            r#""type":"0x1","status":"0x2","root":"ROOT""#,
            r#""type":"0x1","status":"0x1","root":{"a":1}"#,
            // unknown and escaped keys
            r#""type":"0x1","status":"0x1","blockHash":"ROOT","gasUsed":"0x1","other":{"status":"0x0"}"#,
            r#""\u0074ype":"0x1","stat\u0075s":"0x1","oth\u0065r":1"#,
            // repeated keys
            r#""type":"0x1","type":"0x1","status":"0x1""#,
            r#""type":"0x1","status":"0x1","status":"0x1""#,
            r#""type":"0x1","status":"0x1","status":"0x1","root":"ROOT""#,
            r#""type":"0x1","status":"0x1","root":"ROOT","root":"0x1""#,
            r#""type":"0x1","status":"0x1","logs":[]"#,
            r#""type":"0x1","status":"0x1","logsBloom":"BLOOM""#,
            r#""type":"0x1","status":"0x1","cumulativeGasUsed":"0x1""#,
        ];
        let documents = cases
            .iter()
            .map(|fields| {
                format!(
                    r#"{{"cumulativeGasUsed":"0x1","logs":[LOG],"logsBloom":"BLOOM",{fields}}}"#
                )
            })
            // Not a receipt with all of its fields.
            .chain([
                r#"{"type":"0x1","status":"0x1","logs":[LOG],"logsBloom":"BLOOM"}"#.to_string(),
                r#"{"type":"0x1","status":"0x1","cumulativeGasUsed":"0x1","logsBloom":"BLOOM"}"#
                    .to_string(),
                r#"{"type":"0x1","status":"0x1","cumulativeGasUsed":"0x1","logs":[LOG]}"#
                    .to_string(),
            ]);

        let message =
            |err: serde_json::Error| err.to_string().split(" at line ").next().unwrap().to_string();
        let from_flattened = |flattened: Flattened| AnyReceiptEnvelope {
            inner: flattened.inner,
            r#type: flattened.ty,
        };

        let mut decoded = 0;
        for json in documents {
            let json = json
                .replace("BLOOM", &Bloom::repeat_byte(0x11).to_string())
                .replace("ROOT", ROOT)
                .replace("LOG", LOG);

            let expected =
                serde_json::from_str::<Flattened>(&json).map(from_flattened).map_err(message);
            decoded += usize::from(expected.is_ok());
            assert_eq!(
                serde_json::from_str::<AnyReceiptEnvelope>(&json).map_err(message),
                expected,
                "{json}"
            );

            // The receipt may be flattened into a larger object, which passes down all its keys.
            assert_eq!(
                serde_json::from_str::<Outer<AnyReceiptEnvelope>>(&json)
                    .map(|outer| outer.receipt)
                    .map_err(message),
                serde_json::from_str::<Outer<Flattened>>(&json)
                    .map(|outer| from_flattened(outer.receipt))
                    .map_err(message),
                "{json}"
            );

            let value = serde_json::from_str::<serde_json::Value>(&json).unwrap();
            assert_eq!(
                serde_json::from_value::<AnyReceiptEnvelope>(value.clone()).map_err(message),
                serde_json::from_value::<Flattened>(value).map(from_flattened).map_err(message),
                "{json}"
            );
        }
        assert_eq!(decoded, 11);

        // Only objects are receipts, as for the derive.
        for (json, unexpected) in [(r#"["0x1","0x1",[],"0x1"]"#, "sequence"), ("null", "null")] {
            assert!(serde_json::from_str::<Flattened>(json).is_err());
            assert_eq!(
                serde_json::from_str::<AnyReceiptEnvelope>(json).map_err(message),
                Err(format!("invalid type: {unexpected}, expected a receipt object"))
            );
        }
    }
}
