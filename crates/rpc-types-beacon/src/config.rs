//! Types for the beacon config endpoints.
//!
//! See <https://ethereum.github.io/beacon-APIs/#/Config>

use alloc::collections::BTreeMap;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the [`/eth/v1/config/deposit_contract`](https://ethereum.github.io/beacon-APIs/#/Config/getDepositContract) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositContractResponse {
    /// The deposit contract data.
    pub data: DepositContract,
}

/// Deposit contract information.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositContract {
    /// The chain ID of the network the deposit contract is deployed on.
    #[serde_as(as = "DisplayFromStr")]
    pub chain_id: u64,
    /// The address of the deposit contract.
    pub address: Address,
}

/// Response from the [`/eth/v1/config/fork_schedule`](https://ethereum.github.io/beacon-APIs/#/Config/getForkSchedule) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkScheduleResponse {
    /// The list of forks in the fork schedule.
    pub data: Vec<crate::fork::Fork>,
}

/// Response from the [`/eth/v1/config/spec`](https://ethereum.github.io/beacon-APIs/#/Config/getSpec) endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecResponse {
    /// The spec configuration as key-value pairs.
    pub data: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::FixedBytes;

    #[test]
    fn serde_deposit_contract_response() {
        let s = r#"{
            "data": {
                "chain_id": "1",
                "address": "0x00000000219ab540356cbb839cbe05303d7705fa"
            }
        }"#;

        let resp: DepositContractResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.chain_id, 1);
        assert_eq!(
            resp.data.address,
            "0x00000000219ab540356cBB839Cbe05303d7705Fa".parse::<Address>().unwrap()
        );

        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: DepositContractResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(resp, deserialized);
    }

    #[test]
    fn serde_fork_schedule_response() {
        let s = r#"{
            "data": [
                {
                    "previous_version": "0x00000000",
                    "current_version": "0x01000000",
                    "epoch": "0"
                },
                {
                    "previous_version": "0x01000000",
                    "current_version": "0x02000000",
                    "epoch": "74240"
                }
            ]
        }"#;

        let resp: ForkScheduleResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].epoch, 0);
        assert_eq!(resp.data[0].current_version, FixedBytes::from([0x01, 0x00, 0x00, 0x00]));
        assert_eq!(resp.data[1].epoch, 74240);

        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: ForkScheduleResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(resp, deserialized);
    }

    #[test]
    fn serde_spec_response() {
        let s = r#"{
            "data": {
                "MAX_VALIDATORS_PER_COMMITTEE": "2048",
                "SECONDS_PER_SLOT": "12",
                "DEPOSIT_CONTRACT_ADDRESS": "0x00000000219ab540356cbb839cbe05303d7705fa"
            }
        }"#;

        let resp: SpecResponse = serde_json::from_str(s).unwrap();
        assert_eq!(resp.data.len(), 3);
        assert_eq!(resp.data.get("SECONDS_PER_SLOT").unwrap(), "12");
        assert_eq!(resp.data.get("MAX_VALIDATORS_PER_COMMITTEE").unwrap(), "2048");

        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: SpecResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(resp, deserialized);
    }
}
