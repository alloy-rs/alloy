#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

//! Sign-In with Ethereum (EIP-4361) utilities.

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

/// Verification options for SIWE message validation.
#[derive(Clone, Debug, Default)]
pub struct VerificationOpts {
    /// Expected domain field. If provided, verification fails if message domain doesn't match.
    pub domain: Option<http::uri::Authority>,
    /// Expected nonce field. If provided, verification fails if message nonce doesn't match.
    pub nonce: Option<String>,
    /// Timestamp to validate against. If provided, time constraints are checked.
    /// For security-sensitive applications, always provide this.
    pub timestamp: Option<time::OffsetDateTime>,
}

#[cfg(feature = "provider")]
mod provider {
    use crate::{Message, VerificationError, VerificationOpts};
    use alloy_eip1271::Eip1271;
    use alloy_primitives::{eip191_hash_message, Address, Signature};
    use alloy_provider::{Network, Provider};

    /// Extension trait for SIWE verification on providers.
    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    pub trait SiweExt<N: Network>: Provider<N> {
        /// Verify a SIWE message signature, with EIP-1271 contract wallet support.
        ///
        /// This method first attempts EIP-191 personal signature verification.
        /// If that fails (e.g., for contract wallets), it falls back to EIP-1271
        /// on-chain verification using the provider.
        ///
        /// # Arguments
        ///
        /// * `message` - The SIWE message to verify
        /// * `signature` - The signature bytes
        /// * `opts` - Verification options (domain, nonce, timestamp checks)
        ///
        /// # Returns
        ///
        /// Returns the verified address on success, or an error if verification fails.
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
            // Validate time constraints
            if let Some(t) = &opts.timestamp {
                if !message.valid_at(t) {
                    return Err(VerificationError::Time);
                }
            }

            // Validate domain
            if let Some(expected_domain) = &opts.domain {
                if *expected_domain != message.domain {
                    return Err(VerificationError::DomainMismatch);
                }
            }

            // Validate nonce
            if let Some(expected_nonce) = &opts.nonce {
                if *expected_nonce != message.nonce {
                    return Err(VerificationError::NonceMismatch);
                }
            }

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
