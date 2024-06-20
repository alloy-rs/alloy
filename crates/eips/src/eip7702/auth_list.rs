use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "k256")]
use alloy_primitives::{keccak256, SignatureError, B256};
use alloy_primitives::{Address, ChainId, Signature};
use alloy_rlp::{BufMut, Decodable, Encodable, Header, RlpEncodable};

/// An EIP-7702 authorization.
#[derive(Debug, Clone, RlpEncodable)]
pub struct Authorization {
    chain_id: ChainId,
    address: Address,
    nonce: OptionalNonce,
    signature: Signature,
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
    /// If this is `Some`, implementers should check that the nonce of the authority (see
    /// [`Self::recover_authority`]) is equal to this nonce.
    pub fn nonce(&self) -> Option<u64> {
        *self.nonce
    }

    /// Get the `signature` for the authorization.
    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Computes the authority prehash used to recover the authority from an authorization list
    /// item.
    ///
    /// The authority prehash is `keccak(MAGIC || rlp([chain_id, [nonce], address]))`
    #[inline]
    #[cfg(feature = "k256")]
    fn authority_prehash(&self) -> B256 {
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

    /// Recover the authority for the authorization.
    ///
    /// # Note
    ///
    /// Implementers should check that the authority has no code.
    #[cfg(feature = "k256")]
    pub fn recover_authority(&self) -> Result<Address, SignatureError> {
        self.signature.recover_address_from_prehash(&self.authority_prehash())
    }
}

/// An internal wrapper around an `Option<u64>` for optional nonces.
///
/// In EIP-7702 the nonce is encoded as a list of either 0 or 1 items, where 0 items means that no
/// nonce was specified (i.e. `None`). If there is 1 item, this is the same as `Some`.
///
/// The wrapper type is used for RLP encoding and decoding.
#[derive(Debug, Copy, Clone)]
struct OptionalNonce(Option<u64>);

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
        let list: Vec<u64> = Vec::decode(buf)?;
        Ok(Self(list.first().copied()))
    }
}

impl Deref for OptionalNonce {
    type Target = Option<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
