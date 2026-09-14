#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
pub use error::{Error, Result, UnsupportedSignerOperation};

mod signer;
pub use signer::{Either, Signer, SignerSync};

pub mod utils;

pub use alloy_primitives::Signature;
pub use k256;

/// Implements chain-ID validation for a signer's `sign_transaction` method.
///
/// When the signer has a chain ID, this fills an unset transaction chain ID or returns
/// [`Error::TransactionChainIdMismatch`] before the signing expression is evaluated. A signer
/// without a chain ID leaves the transaction unchanged. Errors from the signing expression are
/// wrapped with [`Error::other`].
#[macro_export]
macro_rules! sign_transaction_with_chain_id {
    // async (
    //    signer: impl Signer,
    //    tx: &mut impl SignableTransaction<Signature>,
    //    sign: lazy Signature,
    // )
    ($signer:expr, $tx:expr, $sign:expr) => {{
        if let Some(chain_id) = $signer.chain_id() {
            if !$tx.set_chain_id_checked(chain_id) {
                return Err(alloy_signer::Error::TransactionChainIdMismatch {
                    signer: chain_id,
                    // we can only end up here if the tx has a chain id
                    tx: $tx.chain_id().unwrap(),
                });
            }
        }

        $sign.map_err(alloy_signer::Error::other)
    }};
}
