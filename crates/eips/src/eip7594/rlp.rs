use alloc::vec::Vec;
use alloy_rlp::BufMut;

/// A helper trait for encoding [EIP-7594](https://eips.ethereum.org/EIPS/eip-7594) sidecars.
pub trait Encodable7594 {
    /// The length of the 7594 encoded envelope. This is the length of the wrapper
    /// version + the length of the inner encoding.
    fn encode_7594_len(&self) -> usize;

    /// Encode the sidecar according to [EIP-7594] rules. First a 1-byte
    /// wrapper version (if any), then the body of the sidecar.
    ///
    /// [EIP-7594] inner encodings are unspecified, and produce an opaque
    /// bytestring.
    ///
    /// [EIP-7594]: https://eips.ethereum.org/EIPS/eip-7594
    fn encode_7594(&self, out: &mut dyn BufMut);

    /// Encode the sidecar according to [EIP-7594] rules. First a 1-byte
    /// wrapper version (if any), then the body of the sidecar.
    ///
    /// This is a convenience method for encoding into a vec, and returning the
    /// vec.
    fn encoded_7594(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.encode_7594_len());
        self.encode_7594(&mut out);
        out
    }
}

/// A helper trait for decoding [EIP-7594](https://eips.ethereum.org/EIPS/eip-7594) sidecars.
pub trait Decodable7594: Sized {
    /// Decode the sidecar according to [EIP-7594] rules. First a 1-byte
    /// wrapper version (if any), then the body of the sidecar.
    ///
    /// [EIP-7594] inner encodings are unspecified, and produce an opaque
    /// bytestring.
    ///
    /// [EIP-7594]: https://eips.ethereum.org/EIPS/eip-7594
    fn decode_7594(buf: &mut &[u8]) -> alloy_rlp::Result<Self>;
}
