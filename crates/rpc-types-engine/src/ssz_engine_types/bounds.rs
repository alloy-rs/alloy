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
        check(self.len(), 1 << 20, "transactions")?;
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
                    let values = Vec::<T>::from_ssz_bytes(bytes)?;
                    check(values.len(), $limit, stringify!($name))?;
                    Ok(values)
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
