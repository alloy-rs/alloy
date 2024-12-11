use alloy_eips::{
    eip6110::DepositRequest, eip7002::WithdrawalRequest, eip7251::ConsolidationRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Decode, ssz_derive::Encode))]
/// An Electra-compatible execution requests payload.
pub struct ExecutionRequestsV4 {
    /// The requested deposits.
    pub deposits: Vec<DepositRequest>,
    /// The requested withdrawals.
    pub withdrawals: Vec<WithdrawalRequest>,
    /// The requested consolidations.
    pub consolidations: Vec<ConsolidationRequest>,
}
