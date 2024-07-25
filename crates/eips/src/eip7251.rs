//! Contains consolidation types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251): Increase the MAX_EFFECTIVE_BALANCE

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

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
