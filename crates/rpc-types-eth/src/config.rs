use alloy_eips::{eip2124::ForkHash, eip7840::BlobParams};
use alloy_primitives::{Address, U64};
use std::collections::HashMap;

/// Response type for `eth_config`
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EthConfig {
    /// Fork configuration of the current active fork.
    pub current: EthForkConfig,
    /// The `CRC32` hash of the current fork configuration.
    pub current_hash: ForkHash,
    /// The EIP-2124 `CRC32` hash for the current fork of all previous forks
    /// starting from genesis block.
    pub current_fork_id: ForkHash,
    /// Fork configuration of the next scheduled fork.
    pub next: Option<EthForkConfig>,
    /// The `CRC32` hash of the next fork configuration.
    pub next_hash: Option<ForkHash>,
    /// The EIP-2124 `CRC32` hash for the next fork of all previous forks
    /// starting from genesis block.
    pub next_fork_id: Option<ForkHash>,
}

/// The fork configuration object as defined by [`EIP-7910`](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-7910.md).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EthForkConfig {
    /// The fork activation timestamp, represented as a JSON number in Unix epoch seconds (UTC).
    /// For the "current" configuration, this reflects the actual activation time; for "next," it
    /// is the scheduled time. Activation time is required. If a fork is activated at genesis
    /// the value `0` is used. If the fork is not scheduled to be activated or its activation time
    /// is unknown it should not be in the rpc results.
    pub activation_time: u64,
    /// The blob configuration parameters for the specific fork, as defined in the genesis file.
    /// This is a JSON object with three members — `baseFeeUpdateFraction`, `max`, and `target` —
    /// all represented as JSON numbers.
    pub blob_schedule: BlobParams,
    ///     The chain ID of the current network, presented as a string with an unsigned 0x-prefixed
    /// hexadecimal number, with all leading zeros removed. This specification does not support
    /// chains without a chain ID or with a chain ID of zero.
    ///
    /// For purposes of canonicalization this value must always be a string.
    pub chain_id: U64,
    /// A representation of the active precompile contracts for the fork. If a precompile is
    /// replaced by an on-chain contract, or removed, then it is not included.
    ///
    /// This is a JSON object where the members are the 20-byte 0x-prefixed hexadecimal addresses
    /// of the precompiles (with zeros preserved), and the values are agreed-upon names for each
    /// contract, typically specified in the EIP defining that contract.
    ///
    /// For Cancun, the contract names are (in order): `ECREC`, `SHA256`, `RIPEMD160`, `ID`,
    /// `MODEXP`, `BN256_ADD`, `BN256_MUL`, `BN256_PAIRING`, `BLAKE2F`, `KZG_POINT_EVALUATION`.
    ///
    /// For Prague, the added contracts are (in order): `BLS12_G1ADD`, `BLS12_G1MSM`,
    /// `BLS12_G2ADD`, `BLS12_G2MSM`, `BLS12_PAIRING_CHECK`, `BLS12_MAP_FP_TO_G1`,
    /// `BLS12_MAP_FP2_TO_G2`.
    pub precompiles: HashMap<Address, String>,
    /// A JSON object representing system-level contracts relevant to the fork, as introduced in
    /// their defining EIPs. Keys are the contract names (e.g., BEACON_ROOTS_ADDRESS) from the
    /// first EIP where they appeared, sorted alphabetically. Values are 20-byte addresses in
    /// 0x-prefixed hexadecimal form, with leading zeros preserved. Omitted for forks before
    /// Cancun.
    ///
    /// For Cancun the only system contract is `BEACON_ROOTS_ADDRESS`.
    ///
    /// For Prague the system contracts are (in order) `BEACON_ROOTS_ADDRESS`,
    /// `CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS`, `DEPOSIT_CONTRACT_ADDRESS`,
    /// `HISTORY_STORAGE_ADDRESS`, and `WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS`.
    ///
    /// Future forks MUST define the list of system contracts in their meta-EIPs.
    pub system_contracts: HashMap<String, Address>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn hoodie_prague_eth_config() {
        let raw = r#"
            {
                "activationTime": 1742999832,
                "blobSchedule": {
                    "baseFeeUpdateFraction": 5007716,
                    "max": 9,
                    "target": 6
                },
                "chainId": "0x88bb0",
                "precompiles": {
                    "0x0000000000000000000000000000000000000001": "ECREC",
                    "0x0000000000000000000000000000000000000002": "SHA256",
                    "0x0000000000000000000000000000000000000003": "RIPEMD160",
                    "0x0000000000000000000000000000000000000004": "ID",
                    "0x0000000000000000000000000000000000000005": "MODEXP",
                    "0x0000000000000000000000000000000000000006": "BN256_ADD",
                    "0x0000000000000000000000000000000000000007": "BN256_MUL",
                    "0x0000000000000000000000000000000000000008": "BN256_PAIRING",
                    "0x0000000000000000000000000000000000000009": "BLAKE2F",
                    "0x000000000000000000000000000000000000000a": "KZG_POINT_EVALUATION",
                    "0x000000000000000000000000000000000000000b": "BLS12_G1ADD",
                    "0x000000000000000000000000000000000000000c": "BLS12_G1MSM",
                    "0x000000000000000000000000000000000000000d": "BLS12_G2ADD",
                    "0x000000000000000000000000000000000000000e": "BLS12_G2MSM",
                    "0x000000000000000000000000000000000000000f": "BLS12_PAIRING_CHECK",
                    "0x0000000000000000000000000000000000000010": "BLS12_MAP_FP_TO_G1",
                    "0x0000000000000000000000000000000000000011": "BLS12_MAP_FP2_TO_G2"
                },
                "systemContracts": {
                    "BEACON_ROOTS_ADDRESS": "0x000f3df6d732807ef1319fb7b8bb8522d0beac02",
                    "CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS": "0x0000bbddc7ce488642fb579f8b00f3a590007251",
                    "DEPOSIT_CONTRACT_ADDRESS": "0x00000000219ab540356cbb839cbe05303d7705fa",
                    "HISTORY_STORAGE_ADDRESS": "0x0000f90827f1c53a10cb7a02335b175320002935",
                    "WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS": "0x00000961ef480eb55e80d19ad83579a64c007002"
                }
            }
        "#;
        assert_eq!(
            serde_json::to_string(&serde_json::from_str::<EthForkConfig>(raw).unwrap()).unwrap(),
            raw.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        );
    }
}
