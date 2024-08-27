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
#[cfg(feature = "jwt")]
mod jwt;
pub mod payload;
mod transition;

pub use self::{cancun::*, forkchoice::*, identification::*, payload::*, transition::*};

#[cfg(feature = "jwt")]
pub use self::jwt::*;

#[doc(inline)]
pub use alloy_eips::eip6110::DepositRequest as DepositRequestV1;

#[doc(inline)]
pub use alloy_eips::eip7002::WithdrawalRequest as WithdrawalRequestV1;

/// The list of all supported Engine capabilities available over the engine endpoint.
///
/// Latest spec: Prague
pub const CAPABILITIES: &[&str] = &[
    "engine_forkchoiceUpdatedV1",
    "engine_forkchoiceUpdatedV2",
    "engine_forkchoiceUpdatedV3",
    "engine_exchangeTransitionConfigurationV1",
    "engine_getClientVersionV1",
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
    "engine_getPayloadBodiesByHashV2",
    "engine_getPayloadBodiesByRangeV2",
];
