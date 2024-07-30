use alloy_primitives::{Parity, SignatureError, U256};

/// Helper trait used to streamline signatures encoding.
pub trait EncodableSignature: Sized {
    /// Instantiate from v, r, s.
    fn from_rs_and_parity<P: TryInto<Parity, Error = E>, E: Into<SignatureError>>(
        r: U256,
        s: U256,
        parity: P,
    ) -> Result<Self, SignatureError>;

    /// Returns the `r` component of this signature.
    fn r(&self) -> U256;

    /// Returns the `s` component of this signature.
    fn s(&self) -> U256;

    /// Returns the recovery ID as a `u8`.
    fn v(&self) -> Parity;

    /// Sets the recovery ID by normalizing a `v` value.
    fn with_parity<T: Into<Parity>>(self, parity: T) -> Self;

    /// Modifies the recovery ID by applying [EIP-155] to a `v` value.
    ///
    /// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    #[inline]
    fn with_chain_id(self, chain_id: u64) -> Self
    where
        Self: Copy,
    {
        self.with_parity(self.v().with_chain_id(chain_id))
    }

    /// Modifies the recovery ID by dropping any [EIP-155] v value, converting
    /// to a simple parity bool.
    fn with_parity_bool(self) -> Self
    where
        Self: Copy,
    {
        self.with_parity(self.v().to_parity_bool())
    }

    /// Decode an RLP-encoded VRS signature.
    fn decode_rlp_vrs(buf: &mut &[u8]) -> Result<Self, alloy_rlp::Error> {
        use alloy_rlp::Decodable;

        let parity: Parity = Decodable::decode(buf)?;
        let r = Decodable::decode(buf)?;
        let s = Decodable::decode(buf)?;

        Self::from_rs_and_parity(r, s, parity)
            .map_err(|_| alloy_rlp::Error::Custom("attempted to decode invalid field element"))
    }

    /// Length of RLP RS field encoding
    fn rlp_rs_len(&self) -> usize {
        alloy_rlp::Encodable::length(&self.r()) + alloy_rlp::Encodable::length(&self.s())
    }

    /// Length of RLP V field encoding
    fn rlp_vrs_len(&self) -> usize {
        self.rlp_rs_len() + alloy_rlp::Encodable::length(&self.v())
    }

    /// Write R and S to an RLP buffer in progress.
    fn write_rlp_rs(&self, out: &mut dyn alloy_rlp::BufMut) {
        alloy_rlp::Encodable::encode(&self.r(), out);
        alloy_rlp::Encodable::encode(&self.s(), out);
    }

    /// Write the V to an RLP buffer without using EIP-155.
    fn write_rlp_v(&self, out: &mut dyn alloy_rlp::BufMut) {
        alloy_rlp::Encodable::encode(&self.v(), out);
    }

    /// Write the VRS to the output. The V will always be 27 or 28.
    fn write_rlp_vrs(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.write_rlp_v(out);
        self.write_rlp_rs(out);
    }
}

impl EncodableSignature for alloy_primitives::Signature {
    fn from_rs_and_parity<P: TryInto<Parity, Error = E>, E: Into<SignatureError>>(
        r: U256,
        s: U256,
        parity: P,
    ) -> Result<Self, SignatureError> {
        Self::from_rs_and_parity(r, s, parity)
    }

    fn r(&self) -> U256 {
        self.r()
    }

    fn s(&self) -> U256 {
        self.s()
    }

    fn v(&self) -> Parity {
        self.v()
    }

    fn with_parity<T: Into<Parity>>(self, parity: T) -> Self {
        self.with_parity(parity)
    }
}
