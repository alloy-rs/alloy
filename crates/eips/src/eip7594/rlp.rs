//! Field-level sidecar codecs and shared RLP list framing.

use alloc::vec::Vec;
use alloy_rlp::BufMut;

#[cfg(feature = "kzg-sidecar")]
use alloy_primitives::FixedBytes;

/// Selects the blob payload representation used when encoding a blob transaction sidecar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlobSidecarEncoding {
    /// Encode the complete sidecar, including its blob payloads.
    #[default]
    WithBlobs,
    /// Encode an empty blob list while retaining commitments and proofs.
    ///
    /// This is the sidecar representation used by eth/72 `PooledTransactions` responses as
    /// specified by [EIP-8070].
    ///
    /// [EIP-8070]: https://eips.ethereum.org/EIPS/eip-8070
    WithoutBlobs,
}

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

    /// Returns the length of the EIP-7594 encoding for the selected blob representation.
    ///
    /// The default delegates to [`Self::encode_7594_len`] because encodings without blob payloads
    /// only differ for sidecar types that contain blobs.
    fn encode_7594_len_with(&self, _encoding: BlobSidecarEncoding) -> usize {
        self.encode_7594_len()
    }

    /// Encodes the sidecar using the selected blob representation.
    ///
    /// The default delegates to [`Self::encode_7594`] because encodings without blob payloads only
    /// differ for sidecar types that contain blobs.
    fn encode_7594_with(&self, _encoding: BlobSidecarEncoding, out: &mut dyn BufMut) {
        self.encode_7594(out);
    }

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

#[cfg(feature = "kzg-sidecar")]
pub(crate) fn decode_sidecar<T>(
    buf: &mut &[u8],
    decode_fields: impl FnOnce(&mut &[u8]) -> alloy_rlp::Result<T>,
) -> alloy_rlp::Result<T> {
    let header = alloy_rlp::Header::decode(buf)?;
    if !header.list {
        return Err(alloy_rlp::Error::UnexpectedString);
    }
    if buf.len() < header.payload_length {
        return Err(alloy_rlp::Error::InputTooShort);
    }
    let remaining = buf.len();
    let sidecar = decode_fields(buf)?;
    if buf.len() + header.payload_length != remaining {
        return Err(alloy_rlp::Error::UnexpectedLength);
    }

    Ok(sidecar)
}

/// Decodes an RLP list of `N`-byte strings, copying each item once from `buf` into the result.
///
/// Equivalent to `Vec::<FixedBytes<N>>::decode`, which first decodes each item into a temporary
/// and grows the vector one item at a time.
#[cfg(feature = "kzg-sidecar")]
pub(crate) fn decode_fixed_list<const N: usize>(
    buf: &mut &[u8],
) -> alloy_rlp::Result<Vec<FixedBytes<N>>> {
    let mut payload = alloy_rlp::Header::decode_bytes(buf, true)?;
    let mut out = Vec::with_capacity(payload.len() / (N + alloy_rlp::length_of_length(N)));
    while !payload.is_empty() {
        let bytes = alloy_rlp::Header::decode_bytes(&mut payload, false)?;
        let item =
            <&FixedBytes<N>>::try_from(bytes).map_err(|_| alloy_rlp::Error::UnexpectedLength)?;
        out.extend_from_slice(core::slice::from_ref(item));
    }
    Ok(out)
}

#[cfg(all(test, feature = "kzg-sidecar"))]
mod tests {
    use super::*;
    use alloy_rlp::Decodable;

    fn assert_matches_vec_decode<const N: usize>(input: &[u8]) {
        let mut expected_buf = input;
        let expected = Vec::<FixedBytes<N>>::decode(&mut expected_buf);
        let mut buf = input;
        let actual = decode_fixed_list::<N>(&mut buf);
        assert_eq!(actual, expected, "{input:x?}");
        assert_eq!(buf, expected_buf, "{input:x?}");
    }

    #[test]
    fn decode_fixed_list_matches_vec_decode() {
        let items = [FixedBytes::<3>::repeat_byte(1), FixedBytes::repeat_byte(0x80)];
        let valid = alloy_rlp::encode(items.to_vec());
        let mut inputs = vec![
            valid.clone(),
            // Empty list.
            vec![alloy_rlp::EMPTY_LIST_CODE],
            // Trailing bytes after the list.
            [valid.as_slice(), &[0xff]].concat(),
            // Outer string instead of list.
            vec![0x83, 1, 2, 3],
            // Empty input.
            vec![],
            // Item is a list.
            vec![0xc4, 0xc3, 1, 2, 3],
            // Item too short, too long and a non-canonical single byte.
            vec![0xc3, 0x82, 1, 2],
            vec![0xc5, 0x84, 1, 2, 3, 4],
            vec![0xc2, 0x81, 0x05],
            // Item header past the end of the list.
            vec![0xc3, 0x83, 1, 2],
            // Outer length past the end of the input.
            vec![0xc8, 0x83, 1, 2, 3],
            // Non-canonical long-form list header.
            vec![0xf8, 0x04, 0x83, 1, 2, 3],
        ];
        for len in 0..valid.len() {
            inputs.push(valid[..len].to_vec());
        }
        for input in &inputs {
            assert_matches_vec_decode::<3>(input);
        }

        assert_eq!(decode_fixed_list::<3>(&mut valid.as_slice()).unwrap(), items);
    }
}
