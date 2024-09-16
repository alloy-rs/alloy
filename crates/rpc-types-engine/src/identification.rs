//! Client identification: <https://github.com/ethereum/execution-apis/blob/main/src/engine/identification.md>

use alloc::string::{String, ToString};
use core::str::FromStr;

/// This enum defines a standard for specifying a client with just two letters. Clients teams which
/// have a code reserved in this list MUST use this code when identifying themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            Self::BU => "BU",
            Self::EJ => "EJ",
            Self::EG => "EG",
            Self::GE => "GE",
            Self::GR => "GR",
            Self::LH => "LH",
            Self::LS => "LS",
            Self::NM => "NM",
            Self::NB => "NB",
            Self::TK => "TK",
            Self::PM => "PM",
            Self::RH => "RH",
        }
    }

    /// Returns the human readable client name for the given code.
    pub const fn client_name(&self) -> &'static str {
        match self {
            Self::BU => "Besu",
            Self::EJ => "EthereumJS",
            Self::EG => "Erigon",
            Self::GE => "Geth",
            Self::GR => "Grandine",
            Self::LH => "Lighthouse",
            Self::LS => "Lodestar",
            Self::NM => "Nethermind",
            Self::NB => "Nimbus",
            Self::TK => "Teku",
            Self::PM => "Prysm",
            Self::RH => "Reth",
        }
    }
}

impl FromStr for ClientCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BU" => Ok(Self::BU),
            "EJ" => Ok(Self::EJ),
            "EG" => Ok(Self::EG),
            "GE" => Ok(Self::GE),
            "GR" => Ok(Self::GR),
            "LH" => Ok(Self::LH),
            "LS" => Ok(Self::LS),
            "NM" => Ok(Self::NM),
            "NB" => Ok(Self::NB),
            "TK" => Ok(Self::TK),
            "PM" => Ok(Self::PM),
            "RH" => Ok(Self::RH),
            s => Err(s.to_string()),
        }
    }
}

impl core::fmt::Display for ClientCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Contains information which identifies a client implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    #[cfg(feature = "serde")]
    fn client_id_serde() {
        let s = r#"{"code":"RH","name":"Reth","version":"v1.10.8","commit":"fa4ff922"}"#;
        let v: ClientVersionV1 = serde_json::from_str(s).unwrap();
        assert_eq!(v.code, ClientCode::RH);
        assert_eq!(v.name, "Reth");
        assert_eq!(serde_json::to_string(&v).unwrap(), s);
    }
}
