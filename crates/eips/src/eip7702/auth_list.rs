use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, Signature, B256, U256};
use alloy_rlp::{
    length_of_length, BufMut, Decodable, Encodable, Header, Result as RlpResult, RlpDecodable,
    RlpEncodable,
};
use core::hash::{Hash, Hasher};

/// Represents the outcome of an attempt to recover the authority from an authorization.
/// It can either be valid (containing an [`Address`]) or invalid (indicating recovery failure).
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RecoveredAuthority {
    /// Indicates a successfully recovered authority address.
    Valid(Address),
    /// Indicates a failed recovery attempt where no valid address could be recovered.
    Invalid,
}

impl RecoveredAuthority {
    /// Returns an optional address if valid.
    pub const fn address(&self) -> Option<Address> {
        match *self {
            Self::Valid(address) => Some(address),
            Self::Invalid => None,
        }
    }

    /// Returns true if the authority is valid.
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid(_))
    }

    /// Returns true if the authority is invalid.
    pub const fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// An unsigned EIP-7702 authorization.
#[derive(Debug, Clone, Hash, RlpEncodable, RlpDecodable, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct Authorization {
    /// The chain ID of the authorization.
    pub chain_id: U256,
    /// The address of the authorization.
    pub address: Address,
    /// The nonce for the authorization.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
}

impl Authorization {
    /// Get the `chain_id` for the authorization.
    ///
    /// # Note
    ///
    /// Implementers should check that this matches the current `chain_id` *or* is 0.
    pub const fn chain_id(&self) -> U256 {
        self.chain_id
    }

    /// Get the `address` for the authorization.
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// Get the `nonce` for the authorization.
    pub const fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Computes the signature hash used to sign the authorization, or recover the authority from a
    /// signed authorization list item.
    ///
    /// The signature hash is `keccak(MAGIC || rlp([chain_id, address, nonce]))`
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
    pub fn into_recovered(self) -> RecoveredAuthorization {
        let authority_result = self.recover_authority();
        let authority =
            authority_result.map_or(RecoveredAuthority::Invalid, RecoveredAuthority::Valid);

        RecoveredAuthorization { inner: self.inner, authority }
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
    /// The result of the authority recovery process, which can either be a valid address or
    /// indicate a failure.
    authority: RecoveredAuthority,
}

impl RecoveredAuthorization {
    /// Instantiate without performing recovery. This should be used carefully.
    pub const fn new_unchecked(inner: Authorization, authority: RecoveredAuthority) -> Self {
        Self { inner, authority }
    }

    /// Returns an optional address based on the current state of the authority.
    pub const fn authority(&self) -> Option<Address> {
        self.authority.address()
    }

    /// Splits the authorization into parts.
    pub const fn into_parts(self) -> (Authorization, RecoveredAuthority) {
        (self.inner, self.authority)
    }
}

#[cfg(feature = "k256")]
impl From<SignedAuthorization> for RecoveredAuthority {
    fn from(value: SignedAuthorization) -> Self {
        value.into_recovered().authority
    }
}

#[cfg(feature = "k256")]
impl From<SignedAuthorization> for RecoveredAuthorization {
    fn from(value: SignedAuthorization) -> Self {
        value.into_recovered()
    }
}
impl Deref for RecoveredAuthorization {
    type Target = Authorization;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
            chain_id: U256::from(1u64),
            address: Address::left_padding_from(&[6]),
            nonce: 1,
        });
    }

    #[test]
    fn test_encode_decode_signed_auth() {
        let auth = SignedAuthorization {
            inner: Authorization {
                chain_id: U256::from(1u64),
                address: Address::left_padding_from(&[6]),
                nonce: 1,
            },
            signature: Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap(),
        };
        let mut buf = Vec::new();
        auth.encode(&mut buf);

        let expected = "f85a01940000000000000000000000000000000000000006011ba048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a0efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804";
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
