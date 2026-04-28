#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

#[cfg(not(target_family = "wasm"))]
mod ws;
#[cfg(not(target_family = "wasm"))]
pub use ws::{MppEvent, MppHandle, MppWsConnect, NoVoucher, VoucherProvider, VoucherRequest};

// Re-exports for ergonomics.
pub use mpp::{
    client::{
        ws::{WsClientMessage, WsServerMessage},
        PaymentProvider,
    },
    PaymentChallenge, PaymentCredential, Receipt,
};
