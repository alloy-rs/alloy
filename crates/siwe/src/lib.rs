#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;

mod message;
mod parser;
mod timestamp;

#[cfg(feature = "rand")]
mod nonce;

pub use message::{Message, ParseError, VerificationError, Version};
pub use timestamp::TimeStamp;

#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
pub use nonce::generate_nonce;

#[cfg(feature = "provider")]
#[cfg_attr(docsrs, doc(cfg(feature = "provider")))]
pub use provider::*;

/// Verification options for [EIP-4361] message validation.
///
/// [EIP-4361]: https://eips.ethereum.org/EIPS/eip-4361
#[derive(Clone, Debug, Default)]
pub struct VerificationOpts {
    /// Expected domain. Fails verification if message domain doesn't match.
    pub domain: Option<http::uri::Authority>,
    /// Expected nonce. Fails verification if message nonce doesn't match.
    pub nonce: Option<String>,
    /// Timestamp for time constraint validation.
    pub timestamp: Option<time::OffsetDateTime>,
}

#[cfg(feature = "provider")]
mod provider {
    use crate::{Message, VerificationError, VerificationOpts};
    use alloy_eip1271::Eip1271;
    use alloy_primitives::{eip191_hash_message, Address, Signature};
    use alloy_provider::{Network, Provider};

    /// Extension trait for SIWE verification with [EIP-1271] support.
    ///
    /// [EIP-1271]: https://eips.ethereum.org/EIPS/eip-1271
    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    pub trait SiweExt<N: Network>: Provider<N> {
        /// Verifies a SIWE message, trying [EIP-191] first then [EIP-1271].
        ///
        /// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
        /// [EIP-1271]: https://eips.ethereum.org/EIPS/eip-1271
        async fn verify_siwe(
            &self,
            message: &Message,
            signature: &[u8],
            opts: &VerificationOpts,
        ) -> Result<Address, VerificationError>;
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    impl<N, P> SiweExt<N> for P
    where
        N: Network,
        P: Provider<N>,
    {
        async fn verify_siwe(
            &self,
            message: &Message,
            signature: &[u8],
            opts: &VerificationOpts,
        ) -> Result<Address, VerificationError> {
            message.validate(opts)?;

            // Try EIP-191 verification first (for EOA wallets)
            if signature.len() == 65 {
                if let Ok(sig) = Signature::try_from(signature) {
                    if let Ok(addr) = message.verify_eip191(&sig) {
                        return Ok(addr);
                    }
                }
            }

            // Fall back to EIP-1271 verification (for contract wallets)
            let message_str = message.to_string();
            let hash = eip191_hash_message(message_str.as_bytes());

            let is_valid = hash
                .verify(message.address, alloy_primitives::Bytes::copy_from_slice(signature), self)
                .await?;

            if is_valid {
                Ok(message.address)
            } else {
                Err(VerificationError::AddressMismatch {
                    expected: message.address,
                    recovered: Address::ZERO,
                })
            }
        }
    }
}
