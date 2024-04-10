//! [EIP-2718] traits.
//!
//! [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718

#[cfg(not(feature = "std"))]
use crate::alloc::{vec, vec::Vec};

use alloy_primitives::{keccak256, Sealed, B256};
use alloy_rlp::{Buf, BufMut, Header, EMPTY_STRING_CODE};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

// https://eips.ethereum.org/EIPS/eip-2718#transactiontype-only-goes-up-to-0x7f
const TX_TYPE_BYTE_MAX: u8 = 0x7f;

/// [EIP-2718] decoding errors.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Clone, Copy, Debug)]
pub enum Eip2718Error {
    /// Rlp error from [`alloy_rlp`].
    RlpError(alloy_rlp::Error),
    /// Got an unexpected type flag while decoding.
    UnexpectedType(u8),
}

impl Display for Eip2718Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RlpError(err) => write!(f, "{err}"),
            Self::UnexpectedType(t) => write!(f, "Unexpected type flag. Got {t}."),
        }
    }
}

impl From<alloy_rlp::Error> for Eip2718Error {
    fn from(err: alloy_rlp::Error) -> Self {
        Self::RlpError(err)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Eip2718Error {}

/// Decoding trait for [EIP-2718] envelopes. These envelopes wrap a transaction
/// or a receipt with a type flag.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
pub trait Decodable2718: Sized {
    /// Extract the type byte from the buffer, if any. The type byte is the
    /// first byte, provided that that first byte is 0x7f or lower.
    fn extract_type_byte(buf: &mut &[u8]) -> Option<u8> {
        buf.first().copied().filter(|b| *b <= TX_TYPE_BYTE_MAX)
    }

    /// Decode the appropriate variant, based on the type flag.
    ///
    /// This function is invoked by [`Self::decode_2718`] with the type byte, and the tail of the
    /// buffer.
    ///
    /// ## Note
    ///
    /// This should be a simple match block that invokes an inner type's RLP decoder.
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> alloy_rlp::Result<Self>;

    /// Decode the default variant.
    ///
    /// This function is invoked by [`Self::decode_2718`] when no type byte can be extracted.
    fn fallback_decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self>;

    /// Decode an EIP-2718 transaction into a concrete instance
    fn decode_2718(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::extract_type_byte(buf)
            .map(|ty| Self::typed_decode(ty, &mut &buf[1..]))
            .unwrap_or_else(|| Self::fallback_decode(buf))
    }

    /// Decode an EIP-2718 transaction in the network format.
    ///
    /// The network format is the RLP encoded string consisting of the
    /// type-flag prepended to an opaque inner encoding. The inner encoding is
    /// RLP for all current Ethereum transaction types, but may not be in future
    /// versions of the protocol.
    fn network_decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        // Keep the original buffer around by copying it.
        let mut h_decode = *buf;
        let h = Header::decode(&mut h_decode)?;

        // If it's a list, we need to fallback to the legacy decoding.
        if h.list {
            return Self::fallback_decode(buf);
        } else {
            *buf = h_decode;
        }

        let remaining_len = buf.len();

        if remaining_len == 0 || remaining_len < h.payload_length {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        let ty = buf[0];
        buf.advance(1);
        let tx = Self::typed_decode(ty, buf)?;

        let bytes_consumed = remaining_len - buf.len();
        // because Header::decode works for single bytes (including the tx type), returning a
        // string Header with payload_length of 1, we need to make sure this check is only
        // performed for transactions with a string header
        if bytes_consumed != h.payload_length && h_decode[0] > EMPTY_STRING_CODE {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        Ok(tx)
    }
}

/// Encoding trait for [EIP-2718] envelopes. These envelopes wrap a transaction
/// or a receipt with a type flag.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
pub trait Encodable2718: Sized + Send + Sync + 'static {
    /// Return the type flag (if any).
    ///
    /// This should return `None` for the default (legacy) variant of the
    /// envelope.
    fn type_flag(&self) -> Option<u8>;

    /// True if the envelope is the legacy variant.
    fn is_legacy(&self) -> bool {
        matches!(self.type_flag(), None | Some(0))
    }

    /// The length of the 2718 encoded envelope. This is the length of the type
    /// flag + the length of the inner transaction RLP.
    fn encode_2718_len(&self) -> usize;

    /// Encode the transaction according to [EIP-2718] rules. First a 1-byte
    /// type flag in the range 0x0-0x7f, then the body of the transaction.
    ///
    /// This implementation uses RLP for the transaction body. Non-standard
    /// users can override this to use some other serialization scheme.
    ///
    /// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
    fn encode_2718(&self, out: &mut dyn BufMut);

    /// Encode the transaction according to [EIP-2718] rules. First a 1-byte
    /// type flag in the range 0x0-0x7f, then the body of the transaction.
    ///
    /// This is a convenience method for encoding into a vec, and returning the
    /// vec.
    fn encoded_2718(&self) -> Vec<u8> {
        let mut out = vec![];
        self.encode_2718(&mut out);
        out
    }

    /// Compute the hash as committed to in the MPT trie.
    fn trie_hash(&self) -> B256 {
        keccak256(self.encoded_2718())
    }

    /// Seal the encodable, by encoding and hashing it.
    fn seal(self) -> Sealed<Self> {
        let hash = self.trie_hash();
        Sealed::new_unchecked(self, hash)
    }

    /// Return the network encoding. For non-legacy items, this is the RLP
    /// encoding of the bytestring of the 2718 encoding. For legacy items it is
    /// simply the legacy encoding.
    fn network_encode(&self, out: &mut dyn BufMut) {
        if !self.is_legacy() {
            Header { list: false, payload_length: self.encode_2718_len() }.encode(out);
        }

        self.encode_2718(out);
    }
}

/// An [EIP-2718] envelope, blanket implemented for types that impl
/// [`Encodable2718`] and [`Decodable2718`]. This envelope is a wrapper around
/// a transaction, or a receipt, or any other type that is differentiated by an
/// EIP-2718 transaction type.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
pub trait Eip2718Envelope: Decodable2718 + Encodable2718 {}
impl<T> Eip2718Envelope for T where T: Decodable2718 + Encodable2718 {}
