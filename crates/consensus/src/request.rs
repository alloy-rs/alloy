use alloc::vec::Vec;
use alloy_eips::{
    eip6110::DepositRequest,
    eip7002::WithdrawalRequest,
    eip7251::ConsolidationRequest,
    eip7685::{Decodable7685, Eip7685Error, Encodable7685},
};
use alloy_primitives::{bytes, Bytes};
use alloy_rlp::{Decodable, Encodable};
use derive_more::{Deref, DerefMut, From, IntoIterator};

/// Ethereum execution layer requests.
///
/// See also [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Request {
    /// An [EIP-6110] deposit request.
    ///
    /// [EIP-6110]: https://eips.ethereum.org/EIPS/eip-6110
    DepositRequest(DepositRequest),
    /// An [EIP-7002] withdrawal request.
    ///
    /// [EIP-7002]: https://eips.ethereum.org/EIPS/eip-7002
    WithdrawalRequest(WithdrawalRequest),
    /// An [EIP-7251] consolidation request.
    ///
    /// [EIP-7251]: https://eips.ethereum.org/EIPS/eip-7251
    ConsolidationRequest(ConsolidationRequest),
}

impl From<DepositRequest> for Request {
    fn from(v: DepositRequest) -> Self {
        Self::DepositRequest(v)
    }
}

impl From<WithdrawalRequest> for Request {
    fn from(v: WithdrawalRequest) -> Self {
        Self::WithdrawalRequest(v)
    }
}

impl From<ConsolidationRequest> for Request {
    fn from(v: ConsolidationRequest) -> Self {
        Self::ConsolidationRequest(v)
    }
}

impl Request {
    /// Whether this is a [`DepositRequest`].
    pub const fn is_deposit_request(&self) -> bool {
        matches!(self, Self::DepositRequest(_))
    }

    /// Whether this is a [`WithdrawalRequest`].
    pub const fn is_withdrawal_request(&self) -> bool {
        matches!(self, Self::WithdrawalRequest(_))
    }

    /// Whether this is a [`ConsolidationRequest`].
    pub const fn is_consolidation_request(&self) -> bool {
        matches!(self, Self::ConsolidationRequest(_))
    }

    /// Return the inner [`DepositRequest`], or `None` of this is not a deposit request.
    pub const fn as_deposit_request(&self) -> Option<&DepositRequest> {
        match self {
            Self::DepositRequest(req) => Some(req),
            _ => None,
        }
    }

    /// Return the inner [`WithdrawalRequest`], or `None` if this is not a withdrawal request.
    pub const fn as_withdrawal_request(&self) -> Option<&WithdrawalRequest> {
        match self {
            Self::WithdrawalRequest(req) => Some(req),
            _ => None,
        }
    }

    /// Return the inner [`ConsolidationRequest`], or `None` if this is not a consolidation request.
    pub const fn as_consolidation_request(&self) -> Option<&ConsolidationRequest> {
        match self {
            Self::ConsolidationRequest(req) => Some(req),
            _ => None,
        }
    }
}

impl Encodable7685 for Request {
    fn request_type(&self) -> u8 {
        match self {
            Self::DepositRequest(_) => 0,
            Self::WithdrawalRequest(_) => 1,
            Self::ConsolidationRequest(_) => 2,
        }
    }

    fn encode_payload_7685(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::DepositRequest(deposit) => deposit.encode(out),
            Self::WithdrawalRequest(withdrawal) => withdrawal.encode(out),
            Self::ConsolidationRequest(consolidation) => consolidation.encode(out),
        }
    }
}

impl Decodable7685 for Request {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, alloy_eips::eip7685::Eip7685Error> {
        Ok(match ty {
            0 => Self::DepositRequest(DepositRequest::decode(buf)?),
            1 => Self::WithdrawalRequest(WithdrawalRequest::decode(buf)?),
            2 => Self::ConsolidationRequest(ConsolidationRequest::decode(buf)?),
            ty => return Err(Eip7685Error::UnexpectedType(ty)),
        })
    }
}

/// A list of EIP-7685 requests.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Deref, DerefMut, From, IntoIterator)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Requests(pub Vec<Request>);

impl Encodable for Requests {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        let mut h = alloy_rlp::Header { list: true, payload_length: 0 };

        let mut encoded = Vec::new();
        for req in &self.0 {
            let encoded_req = req.encoded_7685();
            h.payload_length += encoded_req.len();
            encoded.push(Bytes::from(encoded_req));
        }

        h.encode(out);
        for req in encoded {
            req.encode(out);
        }
    }
}

impl Decodable for Requests {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(<Vec<Bytes> as Decodable>::decode(buf)?
            .into_iter()
            .map(|bytes| Request::decode_7685(&mut bytes.as_ref()))
            .collect::<Result<Vec<_>, alloy_eips::eip7685::Eip7685Error>>()
            .map(Self)?)
    }
}
