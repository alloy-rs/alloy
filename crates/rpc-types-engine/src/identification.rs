//! Client identification: <https://github.com/ethereum/execution-apis/blob/main/src/engine/identification.md>

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// This enum defines a standard for specifying a client with just two letters. Clients teams which
/// have a code reserved in this list MUST use this code when identifying themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientCode {
    /// Besu
    BU,
    /// EthereumJS
    EJ,
    /// Erigon
    EG,
    /// Geth, go-ethereum
    GE,
    /// Grandine
    GR,
    /// Lighthouse
    LH,
    /// Lodestar
    LS,
    /// Nethermind
    NM,
    /// Nimbus
    NB,
    /// Teku
    TK,
    /// Prysm
    PM,
    /// Reth
    RH,
}

impl ClientCode {
    /// Returns the client identifier as str.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ClientCode::BU => "BU",
            ClientCode::EJ => "EJ",
            ClientCode::EG => "EG",
            ClientCode::GE => "GE",
            ClientCode::GR => "GR",
            ClientCode::LH => "LH",
            ClientCode::LS => "LS",
            ClientCode::NM => "NM",
            ClientCode::NB => "NB",
            ClientCode::TK => "TK",
            ClientCode::PM => "PM",
            ClientCode::RH => "RH",
        }
    }

    /// Returns the human readable client name for the given code.
    pub const fn client_name(&self) -> &'static str {
        match self {
            ClientCode::BU => "Besu",
            ClientCode::EJ => "EthereumJS",
            ClientCode::EG => "Erigon",
            ClientCode::GE => "Geth",
            ClientCode::GR => "Grandine",
            ClientCode::LH => "Lighthouse",
            ClientCode::LS => "Lodestar",
            ClientCode::NM => "Nethermind",
            ClientCode::NB => "Nimbus",
            ClientCode::TK => "Teku",
            ClientCode::PM => "Prysm",
            ClientCode::RH => "Reth",
        }
    }
}

impl FromStr for ClientCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BU" => Ok(ClientCode::BU),
            "EJ" => Ok(ClientCode::EJ),
            "EG" => Ok(ClientCode::EG),
            "GE" => Ok(ClientCode::GE),
            "GR" => Ok(ClientCode::GR),
            "LH" => Ok(ClientCode::LH),
            "LS" => Ok(ClientCode::LS),
            "NM" => Ok(ClientCode::NM),
            "NB" => Ok(ClientCode::NB),
            "TK" => Ok(ClientCode::TK),
            "PM" => Ok(ClientCode::PM),
            "RH" => Ok(ClientCode::RH),
            s => Err(s.to_string()),
        }
    }
}

impl std::fmt::Display for ClientCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Contains information which identifies a client implementation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersionV1 {
    /// Client code, e.g. GE for Geth
    pub code: ClientCode,
    /// Human-readable name of the client, e.g. Lighthouse or go-ethereum
    pub name: String,
    /// The version string of the current implementation e.g. v4.6.0 or 1.0.0-alpha.1 or
    /// 1.0.0+20130313144700
    pub version: String,
    /// first four bytes of the latest commit hash of this build e.g: `fa4ff922`
    pub commit: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_serde() {
        let s = r#"{"code":"RH","name":"Reth","version":"v1.10.8","commit":"fa4ff922"}"#;
        let v: ClientVersionV1 = serde_json::from_str(s).unwrap();
        assert_eq!(v.code, ClientCode::RH);
        assert_eq!(v.name, "Reth");
        assert_eq!(serde_json::to_string(&v).unwrap(), s);
    }
}
