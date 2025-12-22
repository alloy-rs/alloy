#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! EIP-1271 smart contract signature verification.
//!
//! This crate provides utilities for verifying signatures using the EIP-1271 standard,
//! which allows smart contracts to validate signatures on behalf of contract accounts.
//!
//! # Example
//!
//! ```ignore
//! use alloy_eip1271::Eip1271;
//! use alloy_primitives::{b256, address, bytes};
//!
//! let hash = b256!("...");
//! let address = address!("...");
//! let signature = bytes!("...");
//!
//! let valid = hash.verify(address, signature, &provider).await?;
//! ```

use alloy_primitives::{Address, Bytes, FixedBytes, B256};
use alloy_provider::{Network, Provider};
use alloy_sol_types::sol;

/// EIP-1271 magic value returned when a signature is valid.
///
/// This is `bytes4(keccak256("isValidSignature(bytes32,bytes)"))`.
pub const MAGIC_VALUE: FixedBytes<4> = FixedBytes::new([0x16, 0x26, 0xba, 0x7e]);

sol! {
    /// EIP-1271 interface for smart contract signature verification.
    #[sol(rpc)]
    contract ERC1271 {
        /// Returns the magic value `0x1626ba7e` if the signature is valid.
        function isValidSignature(
            bytes32 hash,
            bytes memory signature
        ) external view returns (bytes4 magicValue);
    }
}

/// Error type for EIP-1271 verification.
#[derive(Debug, thiserror::Error)]
pub enum Eip1271Error {
    /// Contract call failed.
    #[error("contract call failed: {0}")]
    ContractCall(#[from] alloy_contract::Error),
    /// Contract returned unexpected data (not EIP-1271 compliant).
    #[error("contract is not EIP-1271 compliant")]
    NonCompliant,
}

/// Extension trait for verifying hashes via EIP-1271.
///
/// This trait extends `B256` to allow verifying that a hash was signed by a
/// smart contract account using the EIP-1271 `isValidSignature` interface.
///
/// # Example
///
/// ```ignore
/// use alloy_eip1271::Eip1271;
///
/// let hash = eip191_hash_message(&message);
/// let valid = hash.verify(contract_address, &signature, &provider).await?;
/// ```
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait Eip1271 {
    /// Verify this hash was signed by the address using EIP-1271.
    ///
    /// Calls `isValidSignature(hash, signature)` on the contract at `address`.
    ///
    /// Returns `Ok(true)` if the contract returns the magic value `0x1626ba7e`.
    /// Returns `Ok(false)` if the contract returns a different value.
    /// Returns `Err` if the contract call fails or returns unexpected data.
    async fn verify<N, P>(
        &self,
        address: Address,
        signature: impl Into<Bytes> + Send,
        provider: &P,
    ) -> Result<bool, Eip1271Error>
    where
        N: Network,
        P: Provider<N>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl Eip1271 for B256 {
    async fn verify<N, P>(
        &self,
        address: Address,
        signature: impl Into<Bytes> + Send,
        provider: &P,
    ) -> Result<bool, Eip1271Error>
    where
        N: Network,
        P: Provider<N>,
    {
        let contract = ERC1271::new(address, provider);
        let result = contract.isValidSignature(*self, signature.into()).call().await;

        match result {
            Ok(magic_value) => Ok(magic_value == MAGIC_VALUE),
            Err(alloy_contract::Error::TransportError(e)) if e.is_error_resp() => {
                // Contract reverted or returned error - signature invalid
                Ok(false)
            }
            Err(alloy_contract::Error::ZeroData(..)) => {
                // Empty return data - address is not a contract or doesn't implement EIP-1271
                Ok(false)
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::sol;

    // Compiled with: forge build --via-ir --optimize (solc 0.8.20)
    sol! {
        /// Mock EIP-1271 contract that always returns valid (0x1626ba7e).
        #[sol(rpc, bytecode = "608080604052346100155760f7908161001a8239f35b5f80fdfe60806004361015600d575f80fd5b5f90813560e01c631626ba7e146021575f80fd5b3460bd57604036600319011260bd576024359067ffffffffffffffff9081831160a5573660238401121560a55782600401359180831160a957601f8301601f19908116603f011682019081118282101760a957604052818152366024838501011160a5578160246020940184830137010152604051630b135d3f60e11b8152602090f35b8380fd5b634e487b7160e01b85526041600452602485fd5b5080fdfea2646970667358221220060b596281cc12881b0c5ea891ff6698661631bddcfe87ce067ba3d8a72f42ad64736f6c63430008140033")]
        contract MockERC1271Valid {
            function isValidSignature(bytes32 hash, bytes memory signature) external pure returns (bytes4);
        }

        /// Mock EIP-1271 contract that always returns invalid (0xffffffff).
        #[sol(rpc, bytecode = "608080604052346100155760f8908161001a8239f35b5f80fdfe60806004361015600d575f80fd5b5f90813560e01c631626ba7e146021575f80fd5b3460be57604036600319011260be576024359067ffffffffffffffff9081831160a6573660238401121560a65782600401359180831160aa57601f8301601f19908116603f011682019081118282101760aa57604052818152366024838501011160a65781602460209401848301370101526040516001600160e01b03198152602090f35b8380fd5b634e487b7160e01b85526041600452602485fd5b5080fdfea26469706673582212202a3dac0d2d6530abdd4ac21647be8561563edc21a83e6270b6aed70aa3bba6e164736f6c63430008140033")]
        contract MockERC1271Invalid {
            function isValidSignature(bytes32 hash, bytes memory signature) external pure returns (bytes4);
        }
    }

    #[test]
    fn test_magic_value() {
        // Verify the magic value is correct
        assert_eq!(MAGIC_VALUE, FixedBytes::new([0x16, 0x26, 0xba, 0x7e]));
    }

    #[tokio::test]
    async fn test_eip1271_valid_signature() {
        use alloy_provider::ProviderBuilder;

        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // Deploy the valid mock contract
        let contract = MockERC1271Valid::deploy(&provider).await.unwrap();

        // Test verification - should return true
        let hash = B256::ZERO;
        let signature = Bytes::from(vec![0u8; 65]);

        let is_valid = hash.verify(*contract.address(), signature, &provider).await.unwrap();
        assert!(is_valid, "Expected valid signature");
    }

    #[tokio::test]
    async fn test_eip1271_invalid_signature() {
        use alloy_provider::ProviderBuilder;

        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // Deploy the invalid mock contract
        let contract = MockERC1271Invalid::deploy(&provider).await.unwrap();

        // Test verification - should return false
        let hash = B256::ZERO;
        let signature = Bytes::from(vec![0u8; 65]);

        let is_valid = hash.verify(*contract.address(), signature, &provider).await.unwrap();
        assert!(!is_valid, "Expected invalid signature");
    }

    #[tokio::test]
    async fn test_eip1271_non_contract_address() {
        use alloy_primitives::address;
        use alloy_provider::ProviderBuilder;

        let provider = ProviderBuilder::new().connect_anvil();

        // Test against a non-contract address
        let non_contract = address!("0000000000000000000000000000000000000001");
        let hash = B256::ZERO;
        let signature = Bytes::from(vec![0u8; 65]);

        let result = hash.verify(non_contract, signature, &provider).await;
        // Should return false (not an error) when contract call fails
        assert!(!result.unwrap(), "Expected false for non-contract address");
    }
}
