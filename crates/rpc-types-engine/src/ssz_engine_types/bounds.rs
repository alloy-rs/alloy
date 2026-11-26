//! REST-SSZ field codecs that preserve Alloy's existing representations and validate schema bounds.
use super::*;
use ssz::{Decode, DecodeError};

pub(super) fn check(actual: usize, max: usize, field: &str) -> Result<(), DecodeError> {
    if actual > max {
        return Err(DecodeError::BytesInvalid(format!(
            "{field} exceeds SSZ bound: {actual} > {max}"
        )));
    }
    Ok(())
}

pub(crate) trait Validate {
    fn validate(&self) -> Result<(), DecodeError>;
}

impl Validate for Vec<Bytes> {
    fn validate(&self) -> Result<(), DecodeError> {
        check(self.len(), MAX_TXS_PER_PAYLOAD, "transactions")?;
        for transaction in self {
            check(transaction.len(), 1 << 30, "transaction")?;
        }
        Ok(())
    }
}

impl Validate for Vec<Withdrawal> {
    fn validate(&self) -> Result<(), DecodeError> {
        check(self.len(), 16, "withdrawals")
    }
}

impl Validate for Bytes {
    fn validate(&self) -> Result<(), DecodeError> {
        check(self.len(), 1 << 30, "block_access_list")
    }
}

impl Validate for Requests {
    fn validate(&self) -> Result<(), DecodeError> {
        check(self.len(), 256, "execution_requests")?;
        for request in self.iter() {
            check(request.len(), 1 << 30, "execution_request")?;
        }
        Ok(())
    }
}

impl Validate for ExecutionPayloadV1 {
    fn validate(&self) -> Result<(), DecodeError> {
        check(self.extra_data.len(), 32, "extra_data")?;
        self.transactions.validate()
    }
}
impl Validate for ExecutionPayloadV2 {
    fn validate(&self) -> Result<(), DecodeError> {
        self.payload_inner.validate()?;
        self.withdrawals.validate()
    }
}
impl Validate for ExecutionPayloadV3 {
    fn validate(&self) -> Result<(), DecodeError> {
        self.payload_inner.validate()
    }
}
impl Validate for ExecutionPayloadV4 {
    fn validate(&self) -> Result<(), DecodeError> {
        self.payload_inner.validate()?;
        self.block_access_list.validate()
    }
}

// All bounded fields here are SSZ variable-size values. Their encoding is unchanged.
pub(crate) mod encode {
    use super::*;
    pub(crate) const fn is_ssz_fixed_len() -> bool {
        false
    }
    pub(crate) const fn ssz_fixed_len() -> usize {
        ssz::BYTES_PER_LENGTH_OFFSET
    }
    pub(crate) fn ssz_bytes_len<T: ssz::Encode>(value: &T) -> usize {
        value.ssz_bytes_len()
    }
    pub(crate) fn ssz_append<T: ssz::Encode>(value: &T, buf: &mut Vec<u8>) {
        value.ssz_append(buf);
    }
}

pub(super) mod decode {
    pub(crate) use super::encode::{is_ssz_fixed_len, ssz_fixed_len};
    use super::*;
    pub(crate) fn from_ssz_bytes<T: Decode + Validate>(bytes: &[u8]) -> Result<T, DecodeError> {
        let value = T::from_ssz_bytes(bytes)?;
        value.validate()?;
        Ok(value)
    }
}

macro_rules! bounded_list {
    ($name:ident, $limit:expr) => {
        pub(super) mod $name {
            pub(crate) use super::encode;
            pub(crate) mod decode {
                pub(crate) use super::super::encode::{is_ssz_fixed_len, ssz_fixed_len};
                use super::super::*;
                pub(crate) fn from_ssz_bytes<T: Decode>(
                    bytes: &[u8],
                ) -> Result<Vec<T>, DecodeError> {
                    if bytes.is_empty() {
                        return Ok(Vec::new());
                    }
                    // Check the count before allocating the list or decoding any items.
                    if T::is_ssz_fixed_len() {
                        check(bytes.len().div_ceil(T::ssz_fixed_len()), $limit, stringify!($name))?;
                        Vec::<T>::from_ssz_bytes(bytes)
                    } else {
                        ssz::decode_list_of_variable_length_items(bytes, Some($limit))
                    }
                }
            }
        }
    };
}

bounded_list!(body_hashes, MAX_BODIES_REQUEST);
bounded_list!(body_entries, MAX_BODIES_REQUEST);
bounded_list!(blob_hashes, MAX_BLOBS_REQUEST);
bounded_list!(blob_entries, MAX_BLOBS_REQUEST);
bounded_list!(cells, 128);
bounded_list!(proofs, 128);

macro_rules! bounded_byte_lists {
    ($name:ident, $limit:expr, $max_bytes:expr) => {
        pub(super) mod $name {
            pub(crate) use super::encode;
            pub(crate) mod decode {
                pub(crate) use super::super::encode::{is_ssz_fixed_len, ssz_fixed_len};
                use super::super::*;
                pub(crate) fn from_ssz_bytes(bytes: &[u8]) -> Result<Vec<Bytes>, DecodeError> {
                    let values: Vec<Bytes> =
                        ssz::decode_list_of_variable_length_items(bytes, Some($limit))?;
                    for value in &values {
                        check(value.len(), $max_bytes, stringify!($name))?;
                    }
                    Ok(values)
                }
            }
        }
    };
}

bounded_byte_lists!(witness_state, MAX_WITNESS_ITEMS, MAX_BYTES_PER_WITNESS_NODE);
bounded_byte_lists!(witness_codes, MAX_WITNESS_ITEMS, MAX_BYTES_PER_CODE);
bounded_byte_lists!(witness_headers, MAX_WITNESS_HEADERS, MAX_BYTES_PER_HEADER);

#[cfg(test)]
mod tests {
    use super::*;
    use ssz::Encode;

    struct MustNotDecode<const FIXED: bool>;

    impl<const FIXED: bool> Decode for MustNotDecode<FIXED> {
        fn is_ssz_fixed_len() -> bool {
            FIXED
        }

        fn ssz_fixed_len() -> usize {
            4
        }

        fn from_ssz_bytes(_: &[u8]) -> Result<Self, DecodeError> {
            panic!("over-limit lists must be rejected before decoding any items");
        }
    }

    #[test]
    fn fixed_lists_reject_before_decoding_items() {
        let bytes = vec![0; (MAX_BLOBS_REQUEST + 1) * 4];
        assert!(blob_hashes::decode::from_ssz_bytes::<MustNotDecode<true>>(&bytes).is_err());
        let bytes = vec![0; (MAX_BODIES_REQUEST + 1) * 4];
        assert!(body_hashes::decode::from_ssz_bytes::<MustNotDecode<true>>(&bytes).is_err());
    }

    #[test]
    fn variable_lists_reject_before_decoding_items() {
        let bytes = vec![Bytes::new(); MAX_BLOBS_REQUEST + 1].as_ssz_bytes();
        assert!(blob_entries::decode::from_ssz_bytes::<MustNotDecode<false>>(&bytes).is_err());
        assert!(cells::decode::from_ssz_bytes::<MustNotDecode<false>>(&bytes).is_err());
        assert!(proofs::decode::from_ssz_bytes::<MustNotDecode<false>>(&bytes).is_err());
        let bytes = vec![Bytes::new(); MAX_BODIES_REQUEST + 1].as_ssz_bytes();
        assert!(body_entries::decode::from_ssz_bytes::<MustNotDecode<false>>(&bytes).is_err());
    }

    #[test]
    fn byte_lists_reject_count_before_item_offsets() {
        type Decoder = fn(&[u8]) -> Result<Vec<Bytes>, DecodeError>;
        for (limit, decode) in [
            (MAX_WITNESS_ITEMS, witness_state::decode::from_ssz_bytes as Decoder),
            (MAX_WITNESS_ITEMS, witness_codes::decode::from_ssz_bytes as Decoder),
            (MAX_WITNESS_HEADERS, witness_headers::decode::from_ssz_bytes as Decoder),
        ] {
            // The first offset declares an over-limit count; the remaining offsets are invalid.
            // The count must be rejected before collecting items or inspecting their offsets.
            let mut bytes = vec![0; (limit + 1) * ssz::BYTES_PER_LENGTH_OFFSET];
            let first_offset = u32::try_from(bytes.len()).unwrap().to_le_bytes();
            bytes[..4].copy_from_slice(&first_offset);
            assert!(matches!(
                decode(&bytes),
                Err(DecodeError::BytesInvalid(message)) if message.contains("exceeds maximum")
            ));
        }
    }

    #[test]
    fn bounded_lists_accept_empty_and_limit() {
        for count in [0, MAX_BLOBS_REQUEST] {
            let fixed = vec![42u16; count];
            assert_eq!(
                blob_hashes::decode::from_ssz_bytes::<u16>(&fixed.as_ssz_bytes()).unwrap(),
                fixed
            );
            let variable = vec![Bytes::new(); count];
            assert_eq!(
                cells::decode::from_ssz_bytes::<Bytes>(&variable.as_ssz_bytes()).unwrap(),
                variable
            );
        }
    }

    #[test]
    fn bounded_lists_reject_malformed_lengths_and_offsets() {
        assert!(blob_hashes::decode::from_ssz_bytes::<u16>(&[0]).is_err());
        for bytes in [
            vec![0],
            vec![0; 4],
            vec![8, 0, 0, 0],
            vec![5, 0, 0, 0, 0],
            vec![8, 0, 0, 0, 4, 0, 0, 0],
        ] {
            assert!(cells::decode::from_ssz_bytes::<Bytes>(&bytes).is_err());
        }
    }
}
