#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod signer;
pub use signer::{TurnkeySigner, TurnkeySignerError};

pub use turnkey_client::{self, TurnkeyClientError, TurnkeyP256ApiKey};

/// Alias for the Turnkey SDK client using the P256 API key stamper.
pub type TurnkeyClient = turnkey_client::TurnkeyClient<TurnkeyP256ApiKey>;

/// Alias for the Turnkey SDK client builder using the P256 API key stamper.
pub type TurnkeyClientBuilder = turnkey_client::TurnkeyClientBuilder<TurnkeyP256ApiKey>;
