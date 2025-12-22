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

mod builder;
mod message;
mod parser;
mod timestamp;

#[cfg(feature = "rand")]
mod nonce;

pub use builder::MessageBuilder;
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
                Err(VerificationError::ContractSignatureInvalid(message.address))
            }
        }
    }
}

#[cfg(all(test, feature = "provider"))]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use alloy_provider::ProviderBuilder;
    use alloy_sol_types::sol;

    const TEST_MESSAGE: &str = r#"localhost:4361 wants you to sign in with your Ethereum account:
0x6Da01670d8fc844e736095918bbE11fE8D564163

SIWE Notepad Example

URI: http://localhost:4361
Version: 1
Chain ID: 1
Nonce: kEWepMt9knR6lWJ6A
Issued At: 2021-12-07T18:28:18.807Z"#;

    #[tokio::test]
    async fn test_provider_verify_siwe_eoa() {
        let provider = ProviderBuilder::new().connect_anvil();

        let message: Message = TEST_MESSAGE.parse().unwrap();
        let sig_bytes = hex!("6228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");

        let opts = VerificationOpts::default();
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &sig_bytes, &opts).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), message.address);
    }

    #[tokio::test]
    async fn test_provider_verify_siwe_eoa_invalid_signature() {
        let provider = ProviderBuilder::new().connect_anvil();

        let message: Message = TEST_MESSAGE.parse().unwrap();
        // Modified signature (first byte changed)
        let sig_bytes = hex!("7228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");

        let opts = VerificationOpts::default();
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &sig_bytes, &opts).await;

        // Should fail - falls back to EIP-1271 which also fails
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_provider_verify_siwe_with_validation_opts() {
        let provider = ProviderBuilder::new().connect_anvil();

        let message: Message = TEST_MESSAGE.parse().unwrap();
        let sig_bytes = hex!("6228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");

        // Test with matching nonce
        let opts = VerificationOpts {
            nonce: Some("kEWepMt9knR6lWJ6A".to_string()),
            ..Default::default()
        };
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &sig_bytes, &opts).await;
        assert!(result.is_ok());

        // Test with mismatched nonce
        let opts = VerificationOpts {
            nonce: Some("wrong_nonce_value".to_string()),
            ..Default::default()
        };
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &sig_bytes, &opts).await;
        assert!(matches!(result, Err(VerificationError::NonceMismatch)));
    }

    // Mock EIP-1271 contract that always returns valid
    sol! {
        #[sol(rpc, bytecode = "608080604052346100155760f7908161001a8239f35b5f80fdfe60806004361015600d575f80fd5b5f90813560e01c631626ba7e146021575f80fd5b3460bd57604036600319011260bd576024359067ffffffffffffffff9081831160a5573660238401121560a55782600401359180831160a957601f8301601f19908116603f011682019081118282101760a957604052818152366024838501011160a5578160246020940184830137010152604051630b135d3f60e11b8152602090f35b8380fd5b634e487b7160e01b85526041600452602485fd5b5080fdfea2646970667358221220060b596281cc12881b0c5ea891ff6698661631bddcfe87ce067ba3d8a72f42ad64736f6c63430008140033")]
        contract MockERC1271Valid {
            function isValidSignature(bytes32 hash, bytes memory signature) external pure returns (bytes4);
        }
    }

    // Mock EIP-1271 contract that always returns invalid
    sol! {
        #[sol(rpc, bytecode = "608080604052346100155760f8908161001a8239f35b5f80fdfe60806004361015600d575f80fd5b5f90813560e01c631626ba7e146021575f80fd5b3460be57604036600319011260be576024359067ffffffffffffffff9081831160a6573660238401121560a65782600401359180831160aa57601f8301601f19908116603f011682019081118282101760aa57604052818152366024838501011160a65781602460209401848301370101526040516001600160e01b03198152602090f35b8380fd5b634e487b7160e01b85526041600452602485fd5b5080fdfea26469706673582212202a3dac0d2d6530abdd4ac21647be8561563edc21a83e6270b6aed70aa3bba6e164736f6c63430008140033")]
        contract MockERC1271Invalid {
            function isValidSignature(bytes32 hash, bytes memory signature) external pure returns (bytes4);
        }
    }

    #[tokio::test]
    async fn test_provider_verify_siwe_eip1271_contract() {
        use time::OffsetDateTime;

        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // Deploy the mock contract
        let contract = MockERC1271Valid::deploy(&provider).await.unwrap();
        let contract_address = *contract.address();

        // Create a message with the contract address
        let message = Message::builder()
            .domain("example.com".parse().unwrap())
            .address(contract_address)
            .uri("https://example.com".parse().unwrap())
            .chain_id(1)
            .nonce("12345678".to_string())
            .issued_at(OffsetDateTime::now_utc().into())
            .build();

        // Any signature works since the mock contract always returns valid
        let signature = vec![0u8; 65];

        let opts = VerificationOpts::default();
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &signature, &opts).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), contract_address);
    }

    #[tokio::test]
    async fn test_provider_verify_siwe_eip1271_invalid() {
        use time::OffsetDateTime;

        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // Deploy the invalid mock contract
        let contract = MockERC1271Invalid::deploy(&provider).await.unwrap();
        let contract_address = *contract.address();

        // Create a message with the contract address
        let message = Message::builder()
            .domain("example.com".parse().unwrap())
            .address(contract_address)
            .uri("https://example.com".parse().unwrap())
            .chain_id(1)
            .nonce("12345678".to_string())
            .issued_at(OffsetDateTime::now_utc().into())
            .build();

        let signature = vec![0u8; 65];

        let opts = VerificationOpts::default();
        let result: Result<_, VerificationError> =
            provider.verify_siwe(&message, &signature, &opts).await;

        assert!(matches!(result, Err(VerificationError::ContractSignatureInvalid(_))));
    }
}
