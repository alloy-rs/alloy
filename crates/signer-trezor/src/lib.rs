#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    // TODO:
    // missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

// TODO: Add tracing.
// #[macro_use]
// extern crate tracing;

// TODO: Needed to pin version.
use protobuf as _;

mod app;
pub use app::TrezorEthereum as Trezor;

mod types;
pub use types::{DerivationType as TrezorHDPath, TrezorError};

use alloy_primitives::Address;
use alloy_signer::{Signature, Signer};
use app::TrezorEthereum;
use async_trait::async_trait;

#[cfg(feature = "eip712")]
use alloy_sol_types::{Eip712Domain, SolStruct};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for TrezorEthereum {
    type Error = TrezorError;

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, Self::Error> {
        self.sign_message(message).await
    }

    #[cfg(TODO)]
    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, Self::Error> {
        let mut tx_with_chain = message.clone();
        if tx_with_chain.chain_id().is_none() {
            // in the case we don't have a chain_id, let's use the signer chain id instead
            tx_with_chain.set_chain_id(self.chain_id);
        }
        self.sign_tx(&tx_with_chain).await
    }

    #[cfg(feature = "eip712")]
    async fn sign_typed_data<T: SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Signature, Self::Error> {
        self.sign_typed_struct(payload, domain).await
    }

    fn address(&self) -> Address {
        self.address
    }

    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }
}
