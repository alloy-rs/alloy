use alloy_primitives::Bloom;
use alloy_rlp::{Buf, BufMut, Header};
use core::fmt;

mod envelope;
pub use envelope::ReceiptEnvelope;

mod receipts;
pub use receipts::{Receipt, ReceiptWithBloom, Receipts};

mod status;
pub use status::Eip658Value;

/// Receipt is the result of a transaction execution.
#[doc(alias = "TransactionReceipt")]
#[auto_impl::auto_impl(&, Arc)]
pub trait TxReceipt: Clone + fmt::Debug + PartialEq + Eq + Send + Sync {
    /// The associated log type.
    type Log;

    /// Returns the status or post state of the transaction.
    ///
    /// ## Note
    ///
    /// Use this method instead of [`TxReceipt::status`] when the transaction
    /// is pre-[EIP-658].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    fn status_or_post_state(&self) -> Eip658Value;

    /// Returns true if the transaction was successful OR if the transaction is
    /// pre-[EIP-658]. Results for transactions before [EIP-658] are not
    /// reliable.
    ///
    /// ## Note
    ///
    /// Caution must be taken when using this method for deep-historical
    /// receipts, as it may not accurately reflect the status of the
    /// transaction. The transaction status is not knowable from the receipt
    /// for transactions before [EIP-658].
    ///
    /// This can be handled using [`TxReceipt::status_or_post_state`].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    fn status(&self) -> bool;

    /// Returns the bloom filter for the logs in the receipt. This operation
    /// may be expensive.
    fn bloom(&self) -> Bloom;

    /// Returns the bloom filter for the logs in the receipt, if it is cheap to
    /// compute.
    fn bloom_cheap(&self) -> Option<Bloom> {
        None
    }

    /// Returns the cumulative gas used in the block after this transaction was executed.
    fn cumulative_gas_used(&self) -> u128;

    /// Returns the logs emitted by this transaction.
    fn logs(&self) -> &[Self::Log];
}

/// Receipt type that knows how to encode and decode itself with a [`Bloom`] value.
pub trait RlpReceipt: Sized {
    /// Returns the length of the RLP encoded receipt fields with the provided bloom filter, without
    /// RLP header.
    fn rlp_encoded_fields_length_with_bloom(&self, bloom: Bloom) -> usize;

    /// RLP encodes the receipt fields with the provided bloom filter, without RLP header.
    fn rlp_encode_fields_with_bloom(&self, bloom: Bloom, out: &mut dyn BufMut);

    /// Returns the RLP header for the receipt payload with the provided bloom filter.
    fn rlp_header_with_bloom(&self, bloom: Bloom) -> alloy_rlp::Header {
        alloy_rlp::Header {
            list: true,
            payload_length: self.rlp_encoded_fields_length_with_bloom(bloom),
        }
    }

    /// Returns the length of the receipt payload with the provided bloom filter.
    fn rlp_encoded_length_with_bloom(&self, bloom: Bloom) -> usize {
        self.rlp_header_with_bloom(bloom).length_with_payload()
    }

    /// RLP encodes the receipt with the provided bloom filter.
    fn rlp_encode_with_bloom(&self, bloom: Bloom, out: &mut dyn BufMut) {
        self.rlp_header_with_bloom(bloom).encode(out);
        self.rlp_encode_fields_with_bloom(bloom, out);
    }

    /// RLP decodes receipt's fields and [`Bloom`] into [`ReceiptWithBloom`] instance.
    ///
    /// Note: this should not decode an RLP header.
    fn rlp_decode_fields_with_bloom(buf: &mut &[u8]) -> alloy_rlp::Result<ReceiptWithBloom<Self>>;

    /// RLP decodes receipt and [`Bloom`] into [`ReceiptWithBloom`] instance.
    fn rlp_decode_with_bloom(buf: &mut &[u8]) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        if header.payload_length > buf.len() {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        let mut fields_buf = &buf[..header.payload_length];
        let this = Self::rlp_decode_fields_with_bloom(&mut fields_buf)?;

        if !fields_buf.is_empty() {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        buf.advance(header.payload_length);

        Ok(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eips::eip2718::Encodable2718;
    use alloy_primitives::{address, b256, bytes, hex, Log, LogData};
    use alloy_rlp::{Decodable, Encodable};

    // Test vector from: https://eips.ethereum.org/EIPS/eip-2481
    #[test]
    fn encode_legacy_receipt() {
        let expected = hex!("f901668001b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f85ff85d940000000000000000000000000000000000000011f842a0000000000000000000000000000000000000000000000000000000000000deada0000000000000000000000000000000000000000000000000000000000000beef830100ff");

        let mut data = vec![];
        let receipt =
            ReceiptEnvelope::Legacy(ReceiptWithBloom {
                receipt: Receipt {
                    cumulative_gas_used: 0x1u128,
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
                    status: false.into(),
                },
                logs_bloom: [0; 256].into(),
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
                    cumulative_gas_used: 0x1u128,
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
                    status: false.into(),
                },
                logs_bloom: [0; 256].into(),
            };

        let receipt = ReceiptWithBloom::decode(&mut &data[..]).unwrap();
        assert_eq!(receipt, expected);
    }

    #[test]
    fn gigantic_receipt() {
        let receipt = Receipt {
            cumulative_gas_used: 16747627,
            status: true.into(),
            logs: vec![
                Log {
                    address: address!("4bf56695415f725e43c3e04354b604bcfb6dfb6e"),
                    data: LogData::new_unchecked(
                        vec![b256!(
                            "c69dc3d7ebff79e41f525be431d5cd3cc08f80eaf0f7819054a726eeb7086eb9"
                        )],
                        vec![1; 0xffffff].into(),
                    ),
                },
                Log {
                    address: address!("faca325c86bf9c2d5b413cd7b90b209be92229c2"),
                    data: LogData::new_unchecked(
                        vec![b256!(
                            "8cca58667b1e9ffa004720ac99a3d61a138181963b294d270d91c53d36402ae2"
                        )],
                        vec![1; 0xffffff].into(),
                    ),
                },
            ],
        }
        .with_bloom();

        let len = receipt.length();
        let mut data = Vec::with_capacity(receipt.length());

        receipt.encode(&mut data);
        assert_eq!(data.len(), len);
        let decoded = ReceiptWithBloom::decode(&mut &data[..]).unwrap();

        // receipt.clone().to_compact(&mut data);
        // let (decoded, _) = Receipt::from_compact(&data[..], data.len());
        assert_eq!(decoded, receipt);
    }
}
