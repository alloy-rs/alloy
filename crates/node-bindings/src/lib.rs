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
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[macro_use]
extern crate tracing;

use alloy_primitives::U256;

pub mod anvil;
pub use anvil::{Anvil, AnvilInstance};

pub mod geth;
pub use geth::{Geth, GethInstance};

/// 1 Ether = 1e18 Wei == 0x0de0b6b3a7640000 Wei
pub const WEI_IN_ETHER: U256 = U256::from_limbs([0x0de0b6b3a7640000, 0x0, 0x0, 0x0]);

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 5.0;
/// The default max priority fee per gas, used in case the base fee is within a threshold.
pub const EIP1559_FEE_ESTIMATION_DEFAULT_PRIORITY_FEE: u64 = 3_000_000_000;
/// The threshold for base fee below which we use the default priority fee, and beyond which we
/// estimate an appropriate value for priority fee.
pub const EIP1559_FEE_ESTIMATION_PRIORITY_FEE_TRIGGER: u64 = 100_000_000_000;
/// The threshold max change/difference (in %) at which we will ignore the fee history values
/// under it.
pub const EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE: i64 = 200;

/// A bit of hack to find an unused TCP port.
///
/// Does not guarantee that the given port is unused after the function exists, just that it was
/// unused before the function started (i.e., it does not reserve a port).
fn unused_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("Failed to create TCP listener to find unused port");

    let local_addr =
        listener.local_addr().expect("Failed to read TCP listener local_addr to find unused port");
    local_addr.port()
}
