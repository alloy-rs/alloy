//! Contains consolidation types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251): Increase the MAX_EFFECTIVE_BALANCE

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

use crate::eip7685::{read_exact, Decodable7685, Eip7685Error, Encodable7685};

/// The address for the EIP-7251 consolidation requests contract:
/// `0x00b42dbF2194e931E80326D950320f7d9Dbeac02`
pub const CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS: Address =
    address!("00b42dbF2194e931E80326D950320f7d9Dbeac02");

/// The code for the EIP-7251 consolidation requests contract.
pub static CONSOLIDATION_REQUEST_PREDEPLOY_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe146098573615156028575f545f5260205ff35b36606014156101445760115f54600182026001905f5b5f82111560595781019083028483029004916001019190603e565b90939004341061014457600154600101600155600354806004026004013381556001015f35815560010160203581556001016040359055600101600355005b6003546002548082038060011160ac575060015b5f5b81811460f15780607402838201600402600401805490600101805490600101805490600101549260601b84529083601401528260340152906054015260010160ae565b9101809214610103579060025561010e565b90505f6002555f6003555b5f548061049d141561011d57505f5b6001546001828201116101325750505f610138565b01600190035b5f555f6001556074025ff35b5f5ffd");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for consolidation requests.
pub const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;

/// This structure maps onto the consolidation request object from [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ConsolidationRequest {
    /// Source address
    pub source_address: Address,
    /// Source public key
    pub source_pubkey: FixedBytes<48>,
    /// Target public key
    pub target_pubkey: FixedBytes<48>,
}

impl Decodable7685 for ConsolidationRequest {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, crate::eip7685::Eip7685Error> {
        Ok(match ty {
            CONSOLIDATION_REQUEST_TYPE => Self {
                source_address: Address::from_slice(read_exact(buf, 20)?),
                source_pubkey: FixedBytes::<48>::from_slice(read_exact(buf, 48)?),
                target_pubkey: FixedBytes::<48>::from_slice(read_exact(buf, 48)?),
            },
            ty => return Err(Eip7685Error::UnexpectedType(ty)),
        })
    }
}

impl Encodable7685 for ConsolidationRequest {
    fn request_type(&self) -> u8 {
        CONSOLIDATION_REQUEST_TYPE
    }

    fn encode_payload_7685(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_slice(self.source_address.as_slice());
        out.put_slice(self.source_pubkey.as_slice());
        out.put_slice(self.target_pubkey.as_slice());
    }
}
