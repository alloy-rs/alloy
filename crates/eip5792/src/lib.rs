#![doc = include_str!("../README.md")]
//! Types for the Wallet Call API.
//!
//! - `wallet_getCapabilities` based on [EIP-5792][eip-5792], with the only capability being
//!   `delegation`.
//! - `wallet_sendTransaction` that can perform sequencer-sponsored [EIP-7702][eip-7702] delegations
//!   and send other sequencer-sponsored transactions on behalf of EOAs with delegated code.
//!
//! [eip-5792]: https://eips.ethereum.org/EIPS/eip-5792
//! [eip-7702]: https://eips.ethereum.org/EIPS/eip-7702
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod call;
pub use call::*;
mod wallet;
pub use wallet::*;
