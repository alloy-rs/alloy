//! Engine API types:
//! <https://github.com/ethereum/execution-apis/blob/main/src/engine/authentication.md>
//! and <https://eips.ethereum.org/EIPS/eip-3675>,
//! following the execution specs <https://github.com/ethereum/execution-apis/tree/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine>.

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod cancun;
mod forkchoice;
mod identification;
mod jwt;
mod optimism;
pub mod payload;
mod transition;

pub use self::{
    cancun::*, forkchoice::*, identification::*, jwt::*, optimism::*, payload::*, transition::*,
};

#[doc(inline)]
pub use alloy_eips::eip6110::DepositRequest as DepositRequestV1;

#[doc(inline)]
pub use alloy_eips::eip7002::WithdrawalRequest as WithdrawalRequestV1;

/// The list of all supported Engine capabilities available over the engine endpoint.
pub const CAPABILITIES: [&str; 14] = [
    "engine_forkchoiceUpdatedV1",
    "engine_forkchoiceUpdatedV2",
    "engine_forkchoiceUpdatedV3",
    "engine_exchangeTransitionConfigurationV1",
    "engine_getPayloadV1",
    "engine_getPayloadV2",
    "engine_getPayloadV3",
    "engine_getPayloadV4",
    "engine_newPayloadV1",
    "engine_newPayloadV2",
    "engine_newPayloadV3",
    "engine_newPayloadV4",
    "engine_getPayloadBodiesByHashV1",
    "engine_getPayloadBodiesByRangeV1",
];
