use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, ChainId, Signature, B256};
use alloy_rlp::{
    length_of_length, BufMut, Decodable, Encodable, Header, Result as RlpResult, RlpDecodable,
    RlpEncodable,
};
use core::hash::{Hash, Hasher};

/// An unsigned EIP-7702 authorization.
#[derive(Debug, Clone, Hash, RlpEncodable, RlpDecodable, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct Authorization {
    /// The chain ID of the authorization.
    pub chain_id: ChainId,
    /// The address of the authorization.
    pub address: Address,
    /// The nonce for the authorization.
    pub nonce: OptionalNonce,
}

impl Authorization {
    /// Get the `chain_id` for the authorization.
    ///
    /// # Note
    ///
    /// Implementers should check that this matches the current `chain_id` *or* is 0.
    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    /// Get the `address` for the authorization.
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// Get the `nonce` for the authorization.
    ///
    /// # Note
    ///
    /// If this is `Some`, implementers should check that the nonce of the authority is equal to
    /// this nonce.
    pub fn nonce(&self) -> Option<u64> {
        *self.nonce
    }

    /// Computes the signature hash used to sign the authorization, or recover the authority from a
    /// signed authorization list item.
    ///
    /// The signature hash is `keccak(MAGIC || rlp([chain_id, [nonce], address]))`
    #[inline]
    pub fn signature_hash(&self) -> B256 {
        use super::constants::MAGIC;

        let mut buf = Vec::new();
        buf.put_u8(MAGIC);
        self.encode(&mut buf);

        keccak256(buf)
    }

    /// Convert to a signed authorization by adding a signature.
    pub const fn into_signed(self, signature: Signature) -> SignedAuthorization {
        SignedAuthorization { inner: self, signature }
    }
}

/// A signed EIP-7702 authorization.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedAuthorization {
    #[cfg_attr(feature = "serde", serde(flatten))]
    inner: Authorization,
    #[cfg_attr(feature = "serde", serde(flatten))]
    signature: Signature,
}

impl SignedAuthorization {
    /// Get the `signature` for the authorization.
    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Splits the authorization into parts.
    pub const fn into_parts(self) -> (Authorization, Signature) {
        (self.inner, self.signature)
    }

    /// Decodes the transaction from RLP bytes, including the signature.
    fn decode_fields(buf: &mut &[u8]) -> RlpResult<Self> {
        Ok(Self {
            inner: Authorization {
                chain_id: Decodable::decode(buf)?,
                address: Decodable::decode(buf)?,
                nonce: Decodable::decode(buf)?,
            },
            signature: Signature::decode_rlp_vrs(buf)?,
        })
    }

    /// Outputs the length of the transaction's fields, without a RLP header.
    fn fields_len(&self) -> usize {
        self.inner.chain_id.length()
            + self.inner.address.length()
            + self.inner.nonce.length()
            + self.signature.rlp_vrs_len()
    }
}

impl Hash for SignedAuthorization {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        self.signature.r().hash(state);
        self.signature.s().hash(state);
        self.signature.v().to_u64().hash(state);
    }
}

impl Decodable for SignedAuthorization {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        Self::decode_fields(buf)
    }
}

impl Encodable for SignedAuthorization {
    fn encode(&self, buf: &mut dyn BufMut) {
        Header { list: true, payload_length: self.fields_len() }.encode(buf);
        self.inner.chain_id.encode(buf);
        self.inner.address.encode(buf);
        self.inner.nonce.encode(buf);
        self.signature.write_rlp_vrs(buf)
    }

    fn length(&self) -> usize {
        let len = self.fields_len();
        len + length_of_length(len)
    }
}

#[cfg(feature = "k256")]
impl SignedAuthorization {
    /// Recover the authority for the authorization.
    ///
    /// # Note
    ///
    /// Implementers should check that the authority has no code.
    pub fn recover_authority(&self) -> Result<Address, alloy_primitives::SignatureError> {
        self.signature.recover_address_from_prehash(&self.inner.signature_hash())
    }

    /// Recover the authority and transform the signed authorization into a
    /// [`RecoveredAuthorization`].
    pub fn try_into_recovered(
        self,
    ) -> Result<RecoveredAuthorization, alloy_primitives::SignatureError> {
        let authority = self.recover_authority()?;
        Ok(RecoveredAuthorization { inner: self.inner, authority })
    }
}

impl Deref for SignedAuthorization {
    type Target = Authorization;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(all(any(test, feature = "arbitrary"), feature = "k256"))]
impl<'a> arbitrary::Arbitrary<'a> for SignedAuthorization {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        use k256::{
            ecdsa::{signature::hazmat::PrehashSigner, SigningKey},
            NonZeroScalar,
        };
        use rand::{rngs::StdRng, SeedableRng};

        let rng_seed = u.arbitrary::<[u8; 32]>()?;
        let mut rand_gen = StdRng::from_seed(rng_seed);
        let signing_key: SigningKey = NonZeroScalar::random(&mut rand_gen).into();

        let inner = u.arbitrary::<Authorization>()?;
        let signature_hash = inner.signature_hash();

        let (recoverable_sig, recovery_id) =
            signing_key.sign_prehash(signature_hash.as_ref()).unwrap();
        let signature = Signature::from_signature_and_parity(recoverable_sig, recovery_id)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        Ok(Self { inner, signature })
    }
}

/// A recovered authorization.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecoveredAuthorization {
    #[cfg_attr(feature = "serde", serde(flatten))]
    inner: Authorization,
    authority: Address,
}

impl RecoveredAuthorization {
    /// Instantiate without performing recovery. This should be used carefully.
    pub const fn new_unchecked(inner: Authorization, authority: Address) -> Self {
        Self { inner, authority }
    }

    /// Get the `authority` for the authorization.
    pub const fn authority(&self) -> Address {
        self.authority
    }

    /// Splits the authorization into parts.
    pub const fn into_parts(self) -> (Authorization, Address) {
        (self.inner, self.authority)
    }
}

#[cfg(feature = "k256")]
impl TryFrom<SignedAuthorization> for RecoveredAuthorization {
    type Error = alloy_primitives::SignatureError;

    fn try_from(value: SignedAuthorization) -> Result<Self, Self::Error> {
        value.try_into_recovered()
    }
}

impl Deref for RecoveredAuthorization {
    type Target = Authorization;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// An internal wrapper around an `Option<u64>` for optional nonces.
///
/// In EIP-7702 the nonce is encoded as a list of either 0 or 1 items, where 0 items means that no
/// nonce was specified (i.e. `None`). If there is 1 item, this is the same as `Some`.
///
/// The wrapper type is used for RLP encoding and decoding.
#[derive(Default, Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct OptionalNonce(Option<u64>);

impl OptionalNonce {
    /// Create a new [`OptionalNonce`]
    pub const fn new(nonce: Option<u64>) -> Self {
        Self(nonce)
    }
}

impl From<Option<u64>> for OptionalNonce {
    fn from(value: Option<u64>) -> Self {
        Self::new(value)
    }
}

impl Encodable for OptionalNonce {
    fn encode(&self, out: &mut dyn BufMut) {
        match self.0 {
            Some(nonce) => {
                Header { list: true, payload_length: nonce.length() }.encode(out);
                nonce.encode(out);
            }
            None => Header { list: true, payload_length: 0 }.encode(out),
        }
    }

    fn length(&self) -> usize {
        self.map(|nonce| nonce.length() + length_of_length(nonce.length())).unwrap_or(1)
    }
}

impl Decodable for OptionalNonce {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut bytes = Header::decode_bytes(buf, true)?;
        if bytes.is_empty() {
            return Ok(Self(None));
        }

        let payload_view = &mut bytes;
        let nonce = u64::decode(payload_view)?;
        if !payload_view.is_empty() {
            // if there's more than 1 item in the nonce list we error
            Err(alloy_rlp::Error::UnexpectedLength)
        } else {
            Ok(Self(Some(nonce)))
        }
    }
}

impl Deref for OptionalNonce {
    type Target = Option<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{hex, Signature};
    use core::str::FromStr;

    fn test_encode_decode_roundtrip(auth: Authorization) {
        let mut buf = Vec::new();
        auth.encode(&mut buf);
        let decoded = Authorization::decode(&mut buf.as_ref()).unwrap();
        assert_eq!(buf.len(), auth.length());
        assert_eq!(decoded, auth);
    }

    #[test]
    fn test_encode_decode_auth() {
        // fully filled
        test_encode_decode_roundtrip(Authorization {
            chain_id: 1u64,
            address: Address::left_padding_from(&[6]),
            nonce: Some(1u64).into(),
        });

        // no nonce
        test_encode_decode_roundtrip(Authorization {
            chain_id: 1u64,
            address: Address::left_padding_from(&[6]),
            nonce: None.into(),
        });
    }

    #[test]
    fn opt_nonce_too_many_elements() {
        let mut buf = Vec::new();
        vec![1u64, 2u64].encode(&mut buf);

        assert_eq!(
            OptionalNonce::decode(&mut buf.as_ref()),
            Err(alloy_rlp::Error::UnexpectedLength)
        )
    }

    #[test]
    fn test_encode_decode_signed_auth() {
        let auth = SignedAuthorization {
            inner: Authorization {
                chain_id: 1u64,
                address: Address::left_padding_from(&[6]),
                nonce: Some(1u64).into(),
            },
            signature: Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap(),
        };
        let mut buf = Vec::new();
        auth.encode(&mut buf);

        let expected = "f85b01940000000000000000000000000000000000000006c1011ba048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a0efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804";
        assert_eq!(hex::encode(&buf), expected);

        let decoded = SignedAuthorization::decode(&mut buf.as_ref()).unwrap();
        assert_eq!(buf.len(), auth.length());
        assert_eq!(decoded, auth);
    }

    #[cfg(all(feature = "arbitrary", feature = "k256"))]
    #[test]
    fn test_arbitrary_auth() {
        use arbitrary::Arbitrary;
        let mut unstructured = arbitrary::Unstructured::new(b"unstructured auth");
        // try this multiple times
        let _auth = SignedAuthorization::arbitrary(&mut unstructured).unwrap();
        let _auth = SignedAuthorization::arbitrary(&mut unstructured).unwrap();
        let _auth = SignedAuthorization::arbitrary(&mut unstructured).unwrap();
        let _auth = SignedAuthorization::arbitrary(&mut unstructured).unwrap();
    }
}
