use alloy_primitives::{map::HashMap, Address, ChainId, U64};
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
// NOTE(onbjerg): We use `U64` to serialize the chain ID as a quantity. This can be changed back to `ChainId` with https://github.com/alloy-rs/alloy/issues/1502
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct WalletCapabilities(pub HashMap<U64, Capabilities>);

impl WalletCapabilities {
    /// Get the capabilities of the wallet API for the specified chain ID.
    pub fn get(&self, chain_id: ChainId) -> Option<&Capabilities> {
        self.0.get(&U64::from(chain_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, map::HashMap, U64};

    #[test]
    fn ser() {
        let caps = WalletCapabilities(HashMap::from_iter([(
            U64::from(0x69420),
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
                U64::from(0x69420),
                Capabilities {
                    delegation: DelegationCapability {
                        addresses: vec![address!("90f79bf6eb2c4f870365e785982e1f101e93b906")],
                    },
                },
            )]))
        );
    }
}
