//! Contains consolidation types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251): Increase the MAX_EFFECTIVE_BALANCE

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// The address for the EIP-7251 consolidation requests contract:
/// `0x00b42dbF2194e931E80326D950320f7d9Dbeac02`
pub const CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS: Address =
    address!("00706203067988Ab3E2A2ab626EdCD6f28bDBbbb");

/// The code for the EIP-7251 consolidation requests contract.
pub static CONSOLIDATION_REQUEST_PREDEPLOY_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe1460a8573615156028575f545f5260205ff35b36606014156101555760115f54600182026001905f5b5f82111560595781019083028483029004916001019190603e565b90939004341061015557600154600101600155600354806004026004013381556001015f358155600101602035815560010160403590553360601b5f5260605f60143760745fa0600101600355005b6003546002548082038060011160bc575060015b5f5b8181146101025780607402838201600402600401805490600101805490600101805490600101549260601b84529083601401528260340152906054015260010160be565b9101809214610114579060025561011f565b90505f6002555f6003555b5f548061049d141561012e57505f5b6001546001828201116101435750505f610149565b01600190035b5f555f6001556074025ff35b5f5ffd");

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
