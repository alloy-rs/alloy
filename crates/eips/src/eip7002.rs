//! Contains the system contract and [WithdrawalRequest] types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7002](https://eips.ethereum.org/EIPS/eip-7002): Execution layer triggerable withdrawals

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// The caller to be used when calling the EIP-7002 withdrawal requests contract at the end of the
/// block.
pub const SYSTEM_ADDRESS: Address = address!("fffffffffffffffffffffffffffffffffffffffe");

/// The address for the EIP-7002 withdrawal requests contract.
pub const WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS: Address =
    address!("00A3ca265EBcb825B45F985A16CEFB49958cE017");

/// The code for the EIP-7002 withdrawal requests contract.
pub static WITHDRAWAL_REQUEST_PREDEPLOY_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe146090573615156028575f545f5260205ff35b366038141561012e5760115f54600182026001905f5b5f82111560595781019083028483029004916001019190603e565b90939004341061012e57600154600101600155600354806003026004013381556001015f3581556001016020359055600101600355005b6003546002548082038060101160a4575060105b5f5b81811460dd5780604c02838201600302600401805490600101805490600101549160601b83528260140152906034015260010160a6565b910180921460ed579060025560f8565b90505f6002555f6003555b5f548061049d141561010757505f5b60015460028282011161011c5750505f610122565b01600290035b5f555f600155604c025ff35b5f5ffd");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for withdrawal requests.
pub const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;

/// Represents an execution layer triggerable withdrawal request.
///
/// See [EIP-7002](https://eips.ethereum.org/EIPS/eip-7002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct WithdrawalRequest {
    /// Address of the source of the exit.
    pub source_address: Address,
    /// Validator public key.
    pub validator_pubkey: FixedBytes<48>,
    /// Amount of withdrawn ether in gwei.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub amount: u64,
}
