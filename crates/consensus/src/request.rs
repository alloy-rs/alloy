use alloy_eips::{
    eip6110::DepositRequest,
    eip7002::WithdrawalRequest,
    eip7685::{Decodable7685, Eip7685Error, Encodable7685},
};
use alloy_rlp::{Decodable, Encodable};

/// Ethereum execution layer requests.
///
/// See also [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Request {
    /// An [EIP-6110] deposit request.
    ///
    /// [EIP-6110]: https://eips.ethereum.org/EIPS/eip-6110
    DepositRequest(DepositRequest),
    /// An [EIP-7002] withdrawal request.
    ///
    /// [EIP-7002]: https://eips.ethereum.org/EIPS/eip-7002
    WithdrawalRequest(WithdrawalRequest),
}

impl Encodable7685 for Request {
    fn request_type(&self) -> u8 {
        match self {
            Self::DepositRequest(_) => 0,
            Self::WithdrawalRequest(_) => 1,
        }
    }

    fn encode_payload_7685(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::DepositRequest(deposit) => deposit.encode(out),
            Self::WithdrawalRequest(withdrawal) => withdrawal.encode(out),
        }
    }
}

impl Decodable7685 for Request {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, alloy_eips::eip7685::Eip7685Error> {
        Ok(match ty {
            0 => Self::DepositRequest(DepositRequest::decode(buf)?),
            1 => Self::WithdrawalRequest(WithdrawalRequest::decode(buf)?),
            ty => return Err(Eip7685Error::UnexpectedType(ty)),
        })
    }
}
