//! Utilities for working with EIP-4844 field elements and implementing
//! [`SidecarCoder`].
//!
//! [`SidecarCoder`]: crate::eip4844::builder::SidecarCoder

#[cfg(feature = "kzg")]
use crate::eip4844::Blob;
use crate::eip4844::{FIELD_ELEMENT_BYTES_USIZE, USABLE_BITS_PER_FIELD_ELEMENT};

/// Determine whether a slice of bytes can be contained in a field element.
pub const fn fits_in_fe(data: &[u8]) -> bool {
    const FIELD_ELEMENT_BYTES_USIZE_PLUS_ONE: usize = FIELD_ELEMENT_BYTES_USIZE + 1;

    match data.len() {
        FIELD_ELEMENT_BYTES_USIZE_PLUS_ONE.. => false,
        FIELD_ELEMENT_BYTES_USIZE => data[0] & 0b1100_0000 == 0, // first two bits must be zero
        _ => true,
    }
}

/// Calculate the number of field elements required to store the given
/// number of bytes.
pub const fn minimum_fe_for_bytes(bytes: usize) -> usize {
    (bytes * 8).div_ceil(USABLE_BITS_PER_FIELD_ELEMENT)
}

/// Calculate the number of field elements required to store the given data.
pub const fn minimum_fe(data: &[u8]) -> usize {
    minimum_fe_for_bytes(data.len())
}

/// Maps a slice of bytes to a blob returning a [`c_kzg::Error`] if the bytes
/// cannot be mapped. This is a helper for sidecar construction, and mimics the
/// exact behavior of [`c_kzg::Error`] as of v2.1.1.
#[cfg(feature = "kzg")]
pub fn bytes_to_blob<B: AsRef<[u8]>>(blob: B) -> Result<Blob, c_kzg::Error> {
    let b_ref = blob.as_ref();
    Blob::try_from(b_ref).map_err(|_| {
        // mimic c_kzg error
        c_kzg::Error::InvalidBytesLength(format!(
            "Invalid byte length. Expected {} got {}",
            crate::eip4844::BYTES_PER_BLOB,
            b_ref.len(),
        ))
    })
}

/// Maps a hex string to a blob returning a [`c_kzg::Error`] if the hex
/// cannot be mapped. This is a helper for sidecar construction, and mimics the
/// exact behavior of [`c_kzg::Error`] as of v2.1.1.
#[cfg(feature = "kzg")]
pub fn hex_to_blob<B: AsRef<str>>(blob: B) -> Result<Blob, c_kzg::Error> {
    let b_ref = blob.as_ref();
    alloy_primitives::hex::decode(b_ref)
        .map_err(|e| c_kzg::Error::InvalidHexFormat(format!("Failed to decode hex: {}", e)))
        .and_then(bytes_to_blob)
}

/// A wrapper for a slice of bytes that is a whole, valid field element.
#[derive(Clone, Copy, Debug)]
pub struct WholeFe<'a>(&'a [u8]);

impl<'a> WholeFe<'a> {
    pub(crate) const fn new_unchecked(data: &'a [u8]) -> Self {
        Self(data)
    }

    /// Instantiate a new `WholeFe` from a slice of bytes, if it is a valid
    /// field element.
    pub const fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() == FIELD_ELEMENT_BYTES_USIZE && fits_in_fe(data) {
            Some(Self::new_unchecked(data))
        } else {
            None
        }
    }
}

impl AsRef<[u8]> for WholeFe<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

#[cfg(test)]
mod test {
    use crate::eip4844::{FIELD_ELEMENTS_PER_BLOB, USABLE_BYTES_PER_BLOB};

    use super::*;
    #[test]
    fn calc_required_fe() {
        assert_eq!(minimum_fe(&[0u8; 32]), 2);
        assert_eq!(minimum_fe(&[0u8; 31]), 1);
        assert_eq!(minimum_fe(&[0u8; 33]), 2);
        assert_eq!(minimum_fe(&[0u8; 64]), 3);
        assert_eq!(minimum_fe(&[0u8; 65]), 3);
        assert_eq!(minimum_fe_for_bytes(USABLE_BYTES_PER_BLOB), FIELD_ELEMENTS_PER_BLOB as usize);
    }

    #[test]
    fn calc_is_valid_field_element() {
        assert!(fits_in_fe(&[0u8; 32]));
        assert!(!fits_in_fe(&[0u8; 33]));

        assert!(WholeFe::new(&[0u8; 32]).is_some());
        assert!(WholeFe::new(&[0u8; 33]).is_none());
        assert!(WholeFe::new(&[0u8; 31]).is_none());
    }
}
