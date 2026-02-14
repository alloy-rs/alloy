#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

use alloy_primitives::FixedBytes;
use constants::{BLS_PUBLIC_KEY_BYTES_LEN, BLS_SIGNATURE_BYTES_LEN};

/// Constants used in the Beacon API.
pub mod constants;

// -- Beacon endpoint types --

/// Types for the [`/eth/v2/beacon/blocks`](https://ethereum.github.io/beacon-APIs/#/Beacon) endpoints.
pub mod block;

/// Types for the [`/eth/v1/beacon/headers`](https://ethereum.github.io/beacon-APIs/#/Beacon) endpoints.
pub mod header;

/// Types for the [`/eth/v1/beacon/states`](https://ethereum.github.io/beacon-APIs/#/Beacon) endpoints.
pub mod state;

/// Types for the [`/eth/v1/beacon/states/{state_id}/fork`](https://ethereum.github.io/beacon-APIs/#/Beacon/getStateFork) endpoint.
pub mod fork;

/// Types for the [`/eth/v1/beacon/states/{state_id}/validators`](https://ethereum.github.io/beacon-APIs/#/Beacon) endpoints.
pub mod validator;

/// Types for the [`/eth/v1/beacon/genesis`](https://ethereum.github.io/beacon-APIs/#/Beacon/getGenesis) endpoint.
pub mod genesis;

/// Types for the beacon block payload and builder API.
pub mod payload;

/// Types for the [`/eth/v1/beacon/blob_sidecars`](https://ethereum.github.io/beacon-APIs/#/Beacon/getBlobSidecars) endpoint.
pub mod sidecar;

// -- Config endpoint types --

/// Types for the [`/eth/v1/config`](https://ethereum.github.io/beacon-APIs/#/Config) endpoints.
pub mod config;

// -- Events --

/// Types for the [`/eth/v1/events`](https://ethereum.github.io/beacon-APIs/#/Events/eventstream) endpoint.
pub mod events;

// -- Node endpoint types --

/// Types for the [`/eth/v1/node`](https://ethereum.github.io/beacon-APIs/#/Node) endpoints.
pub mod node;

// -- Validator endpoint types --

/// Types for the [`/eth/v1/validator/duties`](https://ethereum.github.io/beacon-APIs/#/Validator) endpoints.
pub mod duties;

/// Types for the [`/eth/v1/validator/duties/proposer`](https://ethereum.github.io/beacon-APIs/#/Validator/getProposerDuties) endpoint.
pub mod proposer;

// -- Relay / builder types --

/// Types for the relay and builder API.
///
/// See also <https://flashbots.github.io/relay-specs/>
pub mod relay;

/// Types for execution requests (Electra+).
pub mod requests;

// -- Rewards endpoint types --

/// Types for the [`/eth/v1/beacon/rewards`](https://ethereum.github.io/beacon-APIs/#/Rewards) endpoints.
pub mod rewards;

// -- Internal helpers --

/// Withdrawal serde helpers for the beacon API format.
pub mod withdrawals;

/// BLS signature type
pub type BlsSignature = FixedBytes<BLS_SIGNATURE_BYTES_LEN>;

/// BLS public key type
pub type BlsPublicKey = FixedBytes<BLS_PUBLIC_KEY_BYTES_LEN>;
