//! Contains Deposit types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110): Supply validator deposits on chain
//!
//! Provides validator deposits as a list of deposit operations added to the Execution Layer block.

use alloy_primitives::{address, Address, FixedBytes, B256};
use alloy_rlp::{Buf, RlpDecodable, RlpEncodable};

use crate::eip7685::{read_exact, Decodable7685, Eip7685Error, Encodable7685};

/// Mainnet deposit contract address.
pub const MAINNET_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("00000000219ab540356cbb839cbe05303d7705fa");

/// The [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110) request type for deposit requests.
pub const DEPOSIT_REQUEST_TYPE: u8 = 0x00;

/// This structure maps onto the deposit object from [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DepositRequest {
    /// Validator public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount of ether deposited in gwei
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
    /// Deposit index
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub index: u64,
}

impl Decodable7685 for DepositRequest {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, crate::eip7685::Eip7685Error> {
        Ok(match ty {
            DEPOSIT_REQUEST_TYPE => Self {
                pubkey: FixedBytes::<48>::from_slice(read_exact(buf, 48)?),
                withdrawal_credentials: B256::from_slice(read_exact(buf, 32)?),
                amount: buf.get_u64(),
                signature: FixedBytes::<96>::from_slice(read_exact(buf, 96)?),
                index: buf.get_u64(),
            },
            ty => return Err(Eip7685Error::UnexpectedType(ty)),
        })
    }
}

impl Encodable7685 for DepositRequest {
    fn request_type(&self) -> u8 {
        DEPOSIT_REQUEST_TYPE
    }

    fn encode_payload_7685(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_slice(self.pubkey.as_slice());
        out.put_slice(self.withdrawal_credentials.as_slice());
        out.put_u64(self.amount);
        out.put_slice(self.signature.as_slice());
        out.put_u64(self.index);
    }
}
