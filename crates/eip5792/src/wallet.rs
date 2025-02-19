use alloy_primitives::{map::HashMap, Address, ChainId};
use serde::{Deserialize, Serialize};

/// The capability to perform [EIP-7702][eip-7702] delegations, sponsored by the sequencer.
///
/// The sequencer will only perform delegations, and act on behalf of delegated accounts, if the
/// account delegates to one of the addresses specified within this capability.
///
/// [eip-7702]: https://eips.ethereum.org/EIPS/eip-7702
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DelegationCapability {
    /// A list of valid delegation contracts.
    pub addresses: Vec<Address>,
}

/// Wallet capabilities for a specific chain.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Capabilities {
    /// The capability to delegate.
    pub delegation: DelegationCapability,
}

/// A map of wallet capabilities per chain ID.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct WalletCapabilities(
    #[serde(with = "alloy_serde::quantity::hashmap")] pub HashMap<ChainId, Capabilities>,
);

impl WalletCapabilities {
    /// Get the capabilities of the wallet API for the specified chain ID.
    pub fn get(&self, chain_id: ChainId) -> Option<&Capabilities> {
        self.0.get(&chain_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, map::HashMap};

    #[test]
    fn ser() {
        let caps = WalletCapabilities(HashMap::from_iter([(
            0x69420,
            Capabilities {
                delegation: DelegationCapability {
                    addresses: vec![address!("90f79bf6eb2c4f870365e785982e1f101e93b906")],
                },
            },
        )]));
        assert_eq!(serde_json::to_string(&caps).unwrap(), "{\"0x69420\":{\"delegation\":{\"addresses\":[\"0x90f79bf6eb2c4f870365e785982e1f101e93b906\"]}}}");
    }

    #[test]
    fn de() {
        let caps: WalletCapabilities = serde_json::from_str(
            r#"{
                    "0x69420": {
                        "delegation": {
                            "addresses": ["0x90f79bf6eb2c4f870365e785982e1f101e93b906"]
                        }
                    }
                }"#,
        )
        .expect("could not deser");

        assert_eq!(
            caps,
            WalletCapabilities(HashMap::from_iter([(
                0x69420,
                Capabilities {
                    delegation: DelegationCapability {
                        addresses: vec![address!("90f79bf6eb2c4f870365e785982e1f101e93b906")],
                    },
                },
            )]))
        );
    }

    #[test]
    fn test_get_capabilities() {
        let caps = WalletCapabilities(HashMap::from_iter([(
            0x69420,
            Capabilities {
                delegation: DelegationCapability {
                    addresses: vec![address!("90f79bf6eb2c4f870365e785982e1f101e93b906")],
                },
            },
        )]));

        // Retrieve an existing chain ID.
        let capabilities = caps.get(0x69420);
        assert!(capabilities.is_some());
        assert_eq!(
            capabilities.unwrap().delegation.addresses[0],
            address!("90f79bf6eb2c4f870365e785982e1f101e93b906")
        );

        // Try to retrieve a non-existing chain ID.
        let non_existing_capabilities = caps.get(0x12345);
        assert!(non_existing_capabilities.is_none());
    }

    #[test]
    fn test_capabilities_with_empty_delegation() {
        let caps = WalletCapabilities(HashMap::from_iter([(
            0x12345,
            Capabilities { delegation: DelegationCapability { addresses: vec![] } },
        )]));

        // Verify that delegation exists but contains no addresses.
        let capabilities = caps.get(0x12345).unwrap();
        assert!(capabilities.delegation.addresses.is_empty());

        // Serialize and ensure JSON output is correct.
        let serialized = serde_json::to_string(&caps).unwrap();
        assert_eq!(serialized, "{\"0x12345\":{\"delegation\":{\"addresses\":[]}}}");
    }
}
