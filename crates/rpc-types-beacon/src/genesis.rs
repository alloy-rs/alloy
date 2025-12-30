//! Types for the beacon genesis endpoint.

use alloy_primitives::{FixedBytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Response from the `eth/v1/beacon/genesis` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisResponse {
    /// Container for the genesis data.
    pub data: GenesisData,
}

/// Genesis information for the beacon chain.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisData {
    /// Unix timestamp when the beacon chain started.
    #[serde_as(as = "DisplayFromStr")]
    pub genesis_time: u64,
    /// Root hash of the genesis validator set.
    pub genesis_validators_root: B256,
    /// Fork version at genesis.
    pub genesis_fork_version: FixedBytes<4>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_genesis_response() {
        let s = r#"{
            "data": {
                "genesis_time": "1742213400",
                "genesis_validators_root": "0x212f13fc4df078b6cb7db228f1c8307566dcecf900867401a92023d7ba99cb5f",
                "genesis_fork_version": "0x10000910"
            }
        }"#;

        let genesis_response: GenesisResponse = serde_json::from_str(s).unwrap();

        assert_eq!(genesis_response.data.genesis_time, 1742213400);
        assert_eq!(
            genesis_response.data.genesis_validators_root,
            "0x212f13fc4df078b6cb7db228f1c8307566dcecf900867401a92023d7ba99cb5f"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            genesis_response.data.genesis_fork_version,
            FixedBytes::from([0x10, 0x00, 0x09, 0x10])
        );
    }
}
