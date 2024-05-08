//! [Beacon API](https://ethereum.github.io/beacon-APIs) types
//!
//! Provides all relevant types for the various RPC endpoints, grouped by namespace.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use alloy_primitives::FixedBytes;
use constants::{BLS_PUBLIC_KEY_BYTES_LEN, BLS_SIGNATURE_BYTES_LEN};

pub mod constants;
/// Beacon API events support.
pub mod events;
pub mod header;
pub mod payload;
pub mod sidecar;
pub mod withdrawals;

/// BLS signature type
pub type BlsSignature = FixedBytes<BLS_SIGNATURE_BYTES_LEN>;

/// BLS public key type
pub type BlsPublicKey = FixedBytes<BLS_PUBLIC_KEY_BYTES_LEN>;
