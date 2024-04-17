use alloy_primitives::B64;
use serde::{Deserialize, Serialize};

pub enum ClientCode {
    BU, // besu
    EJ, // ethereumJS
    EG, // erigon
    GE, // go-ethereum
    GR, // grandine
    LH, // lighthouse
    LS, // lodestar
    NM, // nethermind
    NB, // nimbus
    TK, // teku
    PM, // prysm
    RH, // reth
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersionV1 {
    pub code :ClientCode,
    pub name: String,
    pub version: String,
    pub commit : Vec<B64>
}