//! [EIP-7685]: General purpose execution layer requests
//!
//! Contains traits for encoding and decoding EIP-7685 requests, as well as validation functions.
//!
//! [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685

#[cfg(not(feature = "std"))]
use crate::alloc::{vec, vec::Vec};

use alloy_rlp::BufMut;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// [EIP-7685] decoding errors.
///
/// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Eip7685Error {
    /// Rlp error from [`alloy_rlp`].
    RlpError(alloy_rlp::Error),
    /// Got an unexpected request type while decoding.
    UnexpectedType(u8),
    /// There was no request type in the buffer.
    MissingType,
}

impl Display for Eip7685Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RlpError(err) => write!(f, "{err}"),
            Self::UnexpectedType(t) => write!(f, "Unexpected request type. Got {t}."),
            Self::MissingType => write!(f, "There was no type flag"),
        }
    }
}

impl From<alloy_rlp::Error> for Eip7685Error {
    fn from(err: alloy_rlp::Error) -> Self {
        Self::RlpError(err)
    }
}

impl From<Eip7685Error> for alloy_rlp::Error {
    fn from(err: Eip7685Error) -> Self {
        match err {
            Eip7685Error::RlpError(err) => err,
            Eip7685Error::MissingType => Self::Custom("eip7685 decoding failed: missing type"),
            Eip7685Error::UnexpectedType(_) => {
                Self::Custom("eip7685 decoding failed: unexpected type")
            }
        }
    }
}

/// Decoding trait for [EIP-7685] requests. The trait should be implemented for an envelope that
/// wraps each possible request type.
///
/// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
pub trait Decodable7685: Sized {
    /// Extract the type byte from the buffer, if any. The type byte is the
    /// first byte.
    fn extract_type_byte(buf: &mut &[u8]) -> Option<u8> {
        buf.first().copied()
    }

    /// Decode the appropriate variant, based on the request type.
    ///
    /// This function is invoked by [`Self::decode_7685`] with the type byte, and the tail of the
    /// buffer.
    ///
    /// ## Note
    ///
    /// This should be a simple match block that invokes an inner type's decoder. The decoder is
    /// request type dependent.
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, Eip7685Error>;

    /// Decode an EIP-7685 request into a concrete instance
    fn decode_7685(buf: &mut &[u8]) -> Result<Self, Eip7685Error> {
        Self::extract_type_byte(buf)
            .map(|ty| Self::typed_decode(ty, &mut &buf[1..]))
            .unwrap_or(Err(Eip7685Error::MissingType))
    }
}

/// Encoding trait for [EIP-7685] requests. The trait should be implemented for an envelope that
/// wraps each possible request type.
///
/// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
pub trait Encodable7685: Sized + Send + Sync + 'static {
    /// Return the request type.
    fn request_type(&self) -> u8;

    /// Encode the request according to [EIP-7685] rules.
    ///
    /// First a 1-byte flag specifying the request type, then the encoded payload.
    ///
    /// The encoding of the payload is request-type dependent.
    ///
    /// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
    fn encode_7685(&self, out: &mut dyn BufMut) {
        out.put_u8(self.request_type());
        self.encode_payload_7685(out);
    }

    /// Encode the request payload.
    ///
    /// The encoding for the payload is request type dependent.
    fn encode_payload_7685(&self, out: &mut dyn BufMut);

    /// Encode the request according to [EIP-7685] rules.
    ///
    /// First a 1-byte flag specifying the request type, then the encoded payload.
    ///
    /// The encoding of the payload is request-type dependent.
    ///
    /// This is a convenience method for encoding into a vec, and returning the
    /// vec.
    ///
    /// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
    fn encoded_7685(&self) -> Vec<u8> {
        let mut out = vec![];
        self.encode_7685(&mut out);
        out
    }
}

/// An [EIP-7685] request envelope, blanket implemented for types that impl
/// [`Encodable7685`] and [`Decodable7685`]. This envelope is a wrapper around
/// a request, differentiated by the request type.
///
/// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
pub trait Eip7685RequestEnvelope: Decodable7685 + Encodable7685 {}
impl<T> Eip7685RequestEnvelope for T where T: Decodable7685 + Encodable7685 {}
