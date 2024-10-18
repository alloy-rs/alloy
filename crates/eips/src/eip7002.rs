//! Contains the system contract, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7002](https://eips.ethereum.org/EIPS/eip-7002): Execution layer triggerable withdrawals

use alloy_primitives::{address, bytes, Address, Bytes};

/// The caller to be used when calling the EIP-7002 withdrawal requests contract at the end of the
/// block.
pub const SYSTEM_ADDRESS: Address = address!("fffffffffffffffffffffffffffffffffffffffe");

/// The address for the EIP-7002 withdrawal requests contract.
pub const WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS: Address =
    address!("09Fc772D0857550724b07B850a4323f39112aAaA");

/// The code for the EIP-7002 withdrawal requests contract.
pub static WITHDRAWAL_REQUEST_PREDEPLOY_CODE: Bytes = bytes!("   3373fffffffffffffffffffffffffffffffffffffffe1460c7573615156028575f545f5260205ff35b36603814156101f05760115f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff146101f057600182026001905f5b5f821115608057810190830284830290049160010191906065565b9093900434106101f057600154600101600155600354806003026004013381556001015f35815560010160203590553360601b5f5260385f601437604c5fa0600101600355005b6003546002548082038060101160db575060105b5f5b81811461017f5780604c02838201600302600401805490600101805490600101549160601b83528260140152807fffffffffffffffffffffffffffffffff0000000000000000000000000000000016826034015260401c906044018160381c81600701538160301c81600601538160281c81600501538160201c81600401538160181c81600301538160101c81600201538160081c81600101535360010160dd565b9101809214610191579060025561019c565b90505f6002555f6003555b5f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff14156101c957505f5b6001546002828201116101de5750505f6101e4565b01600290035b5f555f600155604c025ff35b5f5ffd");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for withdrawal requests.
pub const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;
