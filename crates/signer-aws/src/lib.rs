#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![recursion_limit = "256"]

#[macro_use]
extern crate tracing;

mod signer;
pub use signer::{AwsSigner, AwsSignerError};

pub use aws_config;
pub use aws_sdk_kms;
