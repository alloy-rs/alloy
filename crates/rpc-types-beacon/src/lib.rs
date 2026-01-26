#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use alloy_primitives::FixedBytes;
use constants::{BLS_PUBLIC_KEY_BYTES_LEN, BLS_SIGNATURE_BYTES_LEN};

/// Constants used in the Beacon API.
pub mod constants;

/// Beacon API events support.
pub mod events;

/// Types and functions related to the signed beacon block.
pub mod block;

/// Types and functions related to the beacon block header.
pub mod header;

/// Types for the beacon `node` endpoints.
pub mod node;

/// Types and functions related to the beacon block payload.
pub mod payload;

/// Types and functions related to the relay mechanism.
pub mod relay;

/// Types and functions related to execution requests.
pub mod requests;

/// Types and functions related to the sidecar.
pub mod sidecar;

/// Types and functions related to withdrawals.
pub mod withdrawals;

/// Types for the beacon genesis endpoint.
pub mod genesis;

/// BLS signature type
pub type BlsSignature = FixedBytes<BLS_SIGNATURE_BYTES_LEN>;

/// BLS public key type
pub type BlsPublicKey = FixedBytes<BLS_PUBLIC_KEY_BYTES_LEN>;
