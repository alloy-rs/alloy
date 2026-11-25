//! Shared REST-SSZ optional codec.

use alloc::vec::Vec;

/// An Engine API v2 SSZ optional encoded as `List[T, 1]`.
///
/// This differs from [`Option<T>`]'s `ethereum_ssz` encoding, which uses an SSZ union.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Optional<T>(Option<T>);

impl<T> Optional<T> {
    /// Creates an absent optional.
    pub const fn none() -> Self {
        Self(None)
    }

    /// Creates a present optional.
    pub const fn some(value: T) -> Self {
        Self(Some(value))
    }

    /// Returns the contained value, if present.
    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Returns true if no value is present.
    pub const fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Returns true if a value is present.
    pub const fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Converts into a Rust optional.
    pub fn into_option(self) -> Option<T> {
        self.0
    }
}

impl<T> From<Option<T>> for Optional<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(Self::none, Self::some)
    }
}

impl<T> From<Optional<T>> for Option<T> {
    fn from(value: Optional<T>) -> Self {
        value.into_option()
    }
}

impl<T: ssz::Encode> ssz::Encode for Optional<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_bytes_len(&self) -> usize {
        self.0.as_ref().map_or(0, |value| {
            value.ssz_bytes_len()
                + if T::is_ssz_fixed_len() { 0 } else { ssz::BYTES_PER_LENGTH_OFFSET }
        })
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        if let Some(value) = &self.0 {
            if !T::is_ssz_fixed_len() {
                buf.extend_from_slice(&(ssz::BYTES_PER_LENGTH_OFFSET as u32).to_le_bytes());
            }
            value.ssz_append(buf);
        }
    }
}

impl<T: ssz::Decode> ssz::Decode for Optional<T> {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        if bytes.is_empty() {
            return Ok(Self::none());
        }
        let value = if T::is_ssz_fixed_len() {
            T::from_ssz_bytes(bytes)?
        } else {
            // A present variable-size value has exactly one canonical list offset.
            if bytes.get(..4) != Some(&[4, 0, 0, 0]) {
                return Err(ssz::DecodeError::BytesInvalid(
                    "optional must contain exactly one value".into(),
                ));
            }
            T::from_ssz_bytes(&bytes[4..])?
        };
        Ok(Self::some(value))
    }
}
