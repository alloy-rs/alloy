//! [EIP-7702] types.
//!
//! [EIP-7702]: https://eips.ethereum.org/EIPS/eip-7702

use alloy_primitives::{Address, ChainId, U256};
use alloy_rlp::{Decodable, Encodable};
use core::mem;

/// A list of [`Authorization`] the current transaction will use
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuthorizationList(pub Vec<Authorization>);

impl AuthorizationList {
    /// Calculates a heuristic for the in-memory size of the [`AuthorizationList`]
    #[inline]
    pub fn size(&self) -> usize {
        // take into account capacity
        self.0.iter().map(Authorization::size).sum::<usize>()
            + self.0.capacity() * mem::size_of::<AuthorizationList>()
    }
}

impl Encodable for AuthorizationList {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let payload = self.0.iter().fold(vec![], |mut a, b| {
            let mut buf = vec![];
            b.encode(&mut buf);
            a.extend_from_slice(&buf);
            a
        });
        let list_header = alloy_rlp::Header { list: true, payload_length: payload.len() };
        list_header.encode(out);
        out.put_slice(&payload);
    }
}

impl Decodable for AuthorizationList {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let rlp_head = alloy_rlp::Header::decode(buf)?;
        if !rlp_head.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let started_len = buf.len();
        let list_len = rlp_head.payload_length;
        let mut consumed = 0;

        let mut authorizations = Vec::new();
        while consumed < list_len {
            let authorization: Authorization = Decodable::decode(buf)?;
            consumed = started_len - buf.len();
            authorizations.push(authorization);
        }

        if consumed != rlp_head.payload_length {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        Ok(AuthorizationList(authorizations))
    }
}

/// Authorizations are used to temporarily set the code of its signer to
/// the code referenced by `address`. These also include a `chain_id` (which
/// can be set to zero and not evaluated) as well as an optional `nonce`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Authorization {
    /// Chain ID
    pub chain_id: ChainId,
    /// The address of the code that will get set to the signer's address
    pub address: Address,
    /// Optional nonce
    pub nonce: Option<u64>,
    /// yParity: Signature Y parity
    pub y_parity: bool,
    /// The R field of the signature
    pub r: U256,
    /// The S field of the signature
    pub s: U256,
}

impl Authorization {
    fn fields_length(&self) -> usize {
        let mut length = 0;
        length += self.chain_id.length();
        length += self.address.length();
        length += self.nonce.map(|n| vec![n]).unwrap_or(vec![]).length();
        length += self.y_parity.length();
        length += self.r.length();
        length += self.s.length();
        length
    }

    /// Calculates a heuristic for the in-memory size of the [`Authorization`]
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<ChainId>()
            + mem::size_of::<Address>()
            + mem::size_of::<Option<u64>>()
            + mem::size_of::<bool>()
            + mem::size_of::<U256>()
            + mem::size_of::<U256>()
    }
}

impl Encodable for Authorization {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let list_header = alloy_rlp::Header { list: true, payload_length: self.fields_length() };
        list_header.encode(out);
        self.chain_id.encode(out);
        self.address.encode(out);
        self.nonce.map(|n| vec![n]).unwrap_or(vec![]).encode(out);
        self.y_parity.encode(out);
        self.r.encode(out);
        self.s.encode(out);
    }
}

impl Decodable for Authorization {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let rlp_head = alloy_rlp::Header::decode(buf)?;
        if !rlp_head.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let started_len = buf.len();
        let chain_id: ChainId = Decodable::decode(buf)?;
        let address: Address = Decodable::decode(buf)?;
        let nonce_list: Vec<u64> = Decodable::decode(buf)?;
        let nonce = nonce_list.first().copied();
        let y_parity = Decodable::decode(buf)?;
        let r = Decodable::decode(buf)?;
        let s = Decodable::decode(buf)?;

        let consumed = started_len - buf.len();
        if consumed != rlp_head.payload_length {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        Ok(Self { chain_id, address, nonce, y_parity, r, s })
    }
}

// TODO(eip7702): add tests
