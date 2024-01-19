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
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(feature = "arbitrary", cfg(feature = "std"))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod eip1559;
pub use eip1559::calc_next_block_base_fee;

pub mod eip2718;

pub mod eip2930;

pub mod eip4788;

pub mod eip4844;
pub use eip4844::{calc_blob_gasprice, calc_excess_blob_gas};

pub mod merge;
