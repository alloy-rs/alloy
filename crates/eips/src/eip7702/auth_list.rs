use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, ChainId, B256};
use alloy_rlp::{BufMut, Decodable, Encodable, Header, RlpDecodable, RlpEncodable};

/// An unsigned EIP-7702 authorization.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

        #[derive(RlpEncodable)]
        struct Auth {
            chain_id: ChainId,
            nonce: OptionalNonce,
            address: Address,
        }

        let mut buf = Vec::new();
        buf.put_u8(MAGIC);

        Auth { chain_id: self.chain_id, nonce: self.nonce, address: self.address }.encode(&mut buf);

        keccak256(buf)
    }

    /// Convert to a signed authorization by adding a signature.
    pub const fn into_signed<S>(self, signature: S) -> SignedAuthorization<S> {
        SignedAuthorization { inner: self, signature }
    }
}

/// A signed EIP-7702 authorization.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedAuthorization<S> {
    inner: Authorization,
    signature: S,
}

impl<S> SignedAuthorization<S> {
    /// Get the `signature` for the authorization.
    pub const fn signature(&self) -> &S {
        &self.signature
    }
}

#[cfg(feature = "k256")]
impl SignedAuthorization<alloy_primitives::Signature> {
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
        let authority = self.recover_authority().ok();
        RecoveredAuthorization { inner: self.inner, authority }
    }
}

impl<S> Deref for SignedAuthorization<S> {
    type Target = Authorization;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A recovered authorization.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecoveredAuthorization {
    inner: Authorization,
    authority: Option<Address>,
}

impl RecoveredAuthorization {
    /// Get the `authority` for the authorization.
    ///
    /// If this is `None`, then the authority could not be recovered.
    pub const fn authority(&self) -> Option<Address> {
        self.authority
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
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
}
