#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod entry;
mod env;
mod error;
mod keystore;
mod lookup;

pub use entry::{EntrySummary, KeyType, TokenLimit, WalletType};
pub use env::{tempo_signer_from_env, ENV_ACCESS_KEY, ENV_PRIVATE_KEY, ENV_ROOT_ACCOUNT};
pub use error::TempoSignerError;
pub use keystore::{default_keys_path, TempoKeystore};
pub use lookup::{TempoAccessKey, TempoLookup};
