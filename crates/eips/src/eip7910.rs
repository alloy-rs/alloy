//! Implementation of [`EIP-7910`](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-7910.md).

use crate::{eip2935, eip4788, eip6110, eip7002, eip7251, eip7840::BlobParams};
use alloc::{borrow::ToOwned, collections::BTreeMap, string::String};
use alloy_primitives::{Address, Bytes};
use core::{fmt, str};

/// Response type for `eth_config`
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EthConfig {
    /// Fork configuration of the current active fork.
    pub current: EthForkConfig,
    /// Fork configuration of the next scheduled fork.
    pub next: Option<EthForkConfig>,
    /// Fork configuration of the last fork (before current).
    pub last: Option<EthForkConfig>,
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
    /// The chain ID of the current network, presented as a string with an unsigned 0x-prefixed
    /// hexadecimal number, with all leading zeros removed. This specification does not support
    /// chains without a chain ID or with a chain ID of zero.
    ///
    /// For purposes of canonicalization this value must always be a string.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub chain_id: u64,
    /// The `FORK_HASH` value as specified in [EIP-6122](https://eips.ethereum.org/EIPS/eip-6122) of the specific fork,
    /// presented as an unsigned 0x-prefixed hexadecimal numbers, with zeros left padded to a four
    /// byte length, in lower case.
    pub fork_id: Bytes,
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
    pub precompiles: BTreeMap<String, Address>,
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
    pub system_contracts: BTreeMap<SystemContract, Address>,
}

/// System-level contracts for [`EthForkConfig`].
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr))]
pub enum SystemContract {
    /// Beacon roots system contract.
    BeaconRoots,
    /// Consolidation requests predeploy system contract.
    ConsolidationRequestPredeploy,
    /// Deposit system contract.
    DepositContract,
    /// History storage system contract.
    HistoryStorage,
    /// Withdrawal requests predeploy system contract.
    WithdrawalRequestPredeploy,
}

impl fmt::Display for SystemContract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::BeaconRoots => "BEACON_ROOTS",
            Self::ConsolidationRequestPredeploy => "CONSOLIDATION_REQUEST_PREDEPLOY",
            Self::DepositContract => "DEPOSIT_CONTRACT",
            Self::HistoryStorage => "HISTORY_STORAGE",
            Self::WithdrawalRequestPredeploy => "WITHDRAWAL_REQUEST_PREDEPLOY",
        };
        write!(f, "{str}_ADDRESS")
    }
}

impl str::FromStr for SystemContract {
    type Err = ParseSystemContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let system_contract = match s {
            "BEACON_ROOTS_ADDRESS" => Self::BeaconRoots,
            "CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS" => Self::ConsolidationRequestPredeploy,
            "DEPOSIT_CONTRACT_ADDRESS" => Self::DepositContract,
            "HISTORY_STORAGE_ADDRESS" => Self::HistoryStorage,
            "WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS" => Self::WithdrawalRequestPredeploy,
            _ => return Err(ParseSystemContractError::Unknown(s.to_owned())),
        };
        Ok(system_contract)
    }
}

impl SystemContract {
    /// Enumeration of all [`SystemContract`] variants.
    pub const ALL: [Self; 5] = [
        Self::BeaconRoots,
        Self::ConsolidationRequestPredeploy,
        Self::DepositContract,
        Self::HistoryStorage,
        Self::WithdrawalRequestPredeploy,
    ];

    /// Returns Cancun system contracts.
    pub const fn cancun() -> [(Self, Address); 1] {
        [(Self::BeaconRoots, eip4788::BEACON_ROOTS_ADDRESS)]
    }

    /// Returns Prague system contracts.
    /// Takes an optional deposit contract address. If it's `None`, mainnet deposit contract address
    /// will be used instead.
    pub fn prague(deposit_contract: Option<Address>) -> [(Self, Address); 4] {
        [
            (Self::HistoryStorage, eip2935::HISTORY_STORAGE_ADDRESS),
            (Self::ConsolidationRequestPredeploy, eip7251::CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS),
            (Self::WithdrawalRequestPredeploy, eip7002::WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS),
            (
                Self::DepositContract,
                deposit_contract.unwrap_or(eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS),
            ),
        ]
    }
}

/// Parse error for [`SystemContract`].
#[derive(Debug, thiserror::Error)]
pub enum ParseSystemContractError {
    /// System contract unknown.
    #[error("unknown system contract: {0}")]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_contract_str() {
        assert_eq!(SystemContract::BeaconRoots.to_string(), "BEACON_ROOTS_ADDRESS");
        assert_eq!(
            SystemContract::ConsolidationRequestPredeploy.to_string(),
            "CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS"
        );
        assert_eq!(SystemContract::DepositContract.to_string(), "DEPOSIT_CONTRACT_ADDRESS");
        assert_eq!(SystemContract::HistoryStorage.to_string(), "HISTORY_STORAGE_ADDRESS");
        assert_eq!(
            SystemContract::WithdrawalRequestPredeploy.to_string(),
            "WITHDRAWAL_REQUEST_PREDEPLOY_ADDRESS"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn system_contract_serde_roundtrip() {
        for contract in SystemContract::ALL {
            assert_eq!(
                contract,
                serde_json::from_value(serde_json::to_value(contract).unwrap()).unwrap()
            );
        }
    }

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
                "forkId": "0x0929e24e",
                "precompiles": {
                    "BLAKE2F": "0x0000000000000000000000000000000000000009",
                    "BLS12_G1ADD": "0x000000000000000000000000000000000000000b",
                    "BLS12_G1MSM": "0x000000000000000000000000000000000000000c",
                    "BLS12_G2ADD": "0x000000000000000000000000000000000000000d",
                    "BLS12_G2MSM": "0x000000000000000000000000000000000000000e",
                    "BLS12_MAP_FP2_TO_G2": "0x0000000000000000000000000000000000000011",
                    "BLS12_MAP_FP_TO_G1": "0x0000000000000000000000000000000000000010",
                    "BLS12_PAIRING_CHECK": "0x000000000000000000000000000000000000000f",
                    "BN254_ADD": "0x0000000000000000000000000000000000000006",
                    "BN254_MUL": "0x0000000000000000000000000000000000000007",
                    "BN254_PAIRING": "0x0000000000000000000000000000000000000008",
                    "ECREC": "0x0000000000000000000000000000000000000001",
                    "ID": "0x0000000000000000000000000000000000000004",
                    "KZG_POINT_EVALUATION": "0x000000000000000000000000000000000000000a",
                    "MODEXP": "0x0000000000000000000000000000000000000005",
                    "RIPEMD160": "0x0000000000000000000000000000000000000003",
                    "SHA256": "0x0000000000000000000000000000000000000002"
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

        let fork_config = serde_json::from_str::<EthForkConfig>(raw).unwrap();
        assert_eq!(
            serde_json::to_string(&fork_config).unwrap(),
            raw.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        );
    }
}
