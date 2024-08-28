use alloy_primitives::Address;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

/// The version of the MEV-share API to use.
#[derive(Deserialize, Debug, Serialize, Clone, Default, PartialEq, Eq)]
pub enum ProtocolVersion {
    #[default]
    #[serde(rename = "beta-1")]
    /// The beta-1 version of the API.
    Beta1,
    /// The 0.1 version of the API.
    #[serde(rename = "v0.1")]
    V0_1,
}

/// Represents information about when a bundle was considered by a builder.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsideredByBuildersAt {
    /// The public key of the builder.
    pub pubkey: String,
    /// The timestamp indicating when the bundle was considered by the builder.
    pub timestamp: String,
}

/// Represents information about when a bundle was sealed by a builder.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedByBuildersAt {
    /// The public key of the builder.
    pub pubkey: String,
    /// The timestamp indicating when the bundle was sealed by the builder.
    pub timestamp: String,
}

/// Requirements for the bundle to be included in the block.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Validity {
    /// Specifies the minimum percent of a given bundle's earnings to redistribute
    /// for it to be included in a builder's block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund: Option<Vec<Refund>>,
    /// Specifies what addresses should receive what percent of the overall refund for this bundle,
    /// if it is enveloped by another bundle (eg. a searcher backrun).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_config: Option<Vec<RefundConfig>>,
}

/// Preferences on what data should be shared about the bundle and its transactions
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Privacy {
    /// Hints on what data should be shared about the bundle and its transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<PrivacyHint>,
    /// Names of the builders that should be allowed to see the bundle/transaction.
    /// <https://github.com/flashbots/dowg/blob/main/builder-registrations.json>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builders: Option<Vec<String>>,
}

/// Hints on what data should be shared about the bundle and its transactions
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivacyHint {
    /// The calldata of the bundle's transactions should be shared.
    pub calldata: bool,
    /// The address of the bundle's transactions should be shared.
    pub contract_address: bool,
    /// The logs of the bundle's transactions should be shared.
    pub logs: bool,
    /// The function selector of the bundle's transactions should be shared.
    pub function_selector: bool,
    /// The hash of the bundle's transactions should be shared.
    pub hash: bool,
    /// The hash of the bundle should be shared.
    pub tx_hash: bool,
}

impl PrivacyHint {
    /// Sets the flag indicating inclusion of calldata and returns the modified `PrivacyHint`
    /// instance.
    pub const fn with_calldata(mut self) -> Self {
        self.calldata = true;
        self
    }

    /// Sets the flag indicating inclusion of contract address and returns the modified
    /// `PrivacyHint` instance.
    pub const fn with_contract_address(mut self) -> Self {
        self.contract_address = true;
        self
    }

    /// Sets the flag indicating inclusion of logs and returns the modified `PrivacyHint` instance.
    pub const fn with_logs(mut self) -> Self {
        self.logs = true;
        self
    }

    /// Sets the flag indicating inclusion of function selector and returns the modified
    /// `PrivacyHint` instance.
    pub const fn with_function_selector(mut self) -> Self {
        self.function_selector = true;
        self
    }

    /// Sets the flag indicating inclusion of hash and returns the modified `PrivacyHint` instance.
    pub const fn with_hash(mut self) -> Self {
        self.hash = true;
        self
    }

    /// Sets the flag indicating inclusion of transaction hash and returns the modified
    /// `PrivacyHint` instance.
    pub const fn with_tx_hash(mut self) -> Self {
        self.tx_hash = true;
        self
    }

    /// Checks if calldata inclusion flag is set.
    pub const fn has_calldata(&self) -> bool {
        self.calldata
    }

    /// Checks if contract address inclusion flag is set.
    pub const fn has_contract_address(&self) -> bool {
        self.contract_address
    }

    /// Checks if logs inclusion flag is set.
    pub const fn has_logs(&self) -> bool {
        self.logs
    }

    /// Checks if function selector inclusion flag is set.
    pub const fn has_function_selector(&self) -> bool {
        self.function_selector
    }

    /// Checks if hash inclusion flag is set.
    pub const fn has_hash(&self) -> bool {
        self.hash
    }

    /// Checks if transaction hash inclusion flag is set.
    pub const fn has_tx_hash(&self) -> bool {
        self.tx_hash
    }

    /// Calculates the number of hints set within the `PrivacyHint` instance.
    const fn num_hints(&self) -> usize {
        let mut num_hints = 0;
        if self.calldata {
            num_hints += 1;
        }
        if self.contract_address {
            num_hints += 1;
        }
        if self.logs {
            num_hints += 1;
        }
        if self.function_selector {
            num_hints += 1;
        }
        if self.hash {
            num_hints += 1;
        }
        if self.tx_hash {
            num_hints += 1;
        }
        num_hints
    }
}

impl Serialize for PrivacyHint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.num_hints()))?;
        if self.calldata {
            seq.serialize_element("calldata")?;
        }
        if self.contract_address {
            seq.serialize_element("contract_address")?;
        }
        if self.logs {
            seq.serialize_element("logs")?;
        }
        if self.function_selector {
            seq.serialize_element("function_selector")?;
        }
        if self.hash {
            seq.serialize_element("hash")?;
        }
        if self.tx_hash {
            seq.serialize_element("tx_hash")?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for PrivacyHint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hints = Vec::<String>::deserialize(deserializer)?;
        let mut privacy_hint = Self::default();
        for hint in hints {
            match hint.as_str() {
                "calldata" => privacy_hint.calldata = true,
                "contract_address" => privacy_hint.contract_address = true,
                "logs" => privacy_hint.logs = true,
                "function_selector" => privacy_hint.function_selector = true,
                "hash" => privacy_hint.hash = true,
                "tx_hash" => privacy_hint.tx_hash = true,
                _ => return Err(serde::de::Error::custom("invalid privacy hint")),
            }
        }
        Ok(privacy_hint)
    }
}

/// Specifies the minimum percent of a given bundle's earnings to redistribute for it to be included
/// in a builder's block.
///
/// Related endpoint: `mev_sendBundle`, `mev_simBundle`, `eth_sendPrivateTransaction`,
/// `eth_sendPrivateRawTransaction`
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    /// The index of the transaction in the bundle.
    #[serde(with = "alloy_serde::quantity")]
    pub body_idx: u64,
    /// The minimum percent of the bundle's earnings to redistribute.
    #[serde(with = "alloy_serde::quantity")]
    pub percent: u64,
}

/// Specifies what addresses should receive what percent of the overall refund for this bundle,
/// if it is enveloped by another bundle (eg. a searcher backrun).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RefundConfig {
    /// The address to refund.
    pub address: Address,
    /// The minimum percent of the bundle's earnings to redistribute.
    #[serde(with = "alloy_serde::quantity")]
    pub percent: u64,
}
