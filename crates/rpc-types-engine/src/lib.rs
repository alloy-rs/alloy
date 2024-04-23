//! Engine API types:
//! <https://github.com/ethereum/execution-apis/blob/main/src/engine/authentication.md>
//! and <https://eips.ethereum.org/EIPS/eip-3675>,
//! following the execution specs <https://github.com/ethereum/execution-apis/tree/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine>.

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod cancun;
mod exit;
mod forkchoice;
mod identification;
mod jwt;
mod optimism;
pub mod payload;
mod transition;

pub use self::{
    cancun::*, exit::*, forkchoice::*, identification::*, jwt::*, optimism::*, payload::*,
    transition::*,
};

#[doc(inline)]
pub use alloy_eips::eip6110::DepositReceipt;

/// The list of all supported Engine capabilities available over the engine endpoint.
pub const CAPABILITIES: [&str; 12] = [
    "engine_forkchoiceUpdatedV1",
    "engine_forkchoiceUpdatedV2",
    "engine_forkchoiceUpdatedV3",
    "engine_exchangeTransitionConfigurationV1",
    "engine_getPayloadV1",
    "engine_getPayloadV2",
    "engine_getPayloadV3",
    "engine_newPayloadV1",
    "engine_newPayloadV2",
    "engine_newPayloadV3",
    "engine_getPayloadBodiesByHashV1",
    "engine_getPayloadBodiesByRangeV1",
];
