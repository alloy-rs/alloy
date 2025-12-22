//! EIP-1271 contract wallet signature verification.

use alloy_primitives::{Address, Bytes, FixedBytes};
use alloy_sol_types::sol;

use crate::VerificationError;

sol! {
    /// EIP-1271 interface for smart contract signature verification.
    #[sol(rpc)]
    contract ERC1271 {
        /// Magic value returned when signature is valid.
        /// bytes4(keccak256("isValidSignature(bytes32,bytes)"))
        bytes4 constant internal MAGIC_VALUE = 0x1626ba7e;

        /// Validates a signature for a given hash.
        ///
        /// @param hash Hash of the data to be signed
        /// @param signature Signature byte array
        /// @return magicValue 0x1626ba7e if valid, 0xffffffff if invalid
        function isValidSignature(
            bytes32 hash,
            bytes memory signature
        ) public view returns (bytes4 magicValue);
    }
}

/// Verify a signature using EIP-1271 contract verification.
///
/// # Arguments
///
/// * `address` - The contract address to verify against
/// * `message_hash` - The hash of the message that was signed
/// * `signature` - The signature bytes
/// * `provider` - The provider to use for the contract call
///
/// # Returns
///
/// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid,
/// or an error if the contract call failed or returned a non-compliant response.
#[cfg(feature = "provider")]
pub async fn verify_eip1271<N, P>(
    address: Address,
    message_hash: FixedBytes<32>,
    signature: Bytes,
    provider: &P,
) -> Result<bool, VerificationError>
where
    N: alloy_provider::Network,
    P: alloy_provider::Provider<N>,
{
    let contract = ERC1271::new(address, provider);
    let result = contract.isValidSignature(message_hash, signature).call().await;

    match result {
        Ok(FixedBytes([0x16, 0x26, 0xba, 0x7e])) => Ok(true),
        Ok(FixedBytes([0xff, 0xff, 0xff, 0xff])) => Ok(false),
        Ok(_) => Err(VerificationError::Eip1271NonCompliant),
        Err(e) => Err(VerificationError::Contract(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_value() {
        // EIP-1271 magic value: bytes4(keccak256("isValidSignature(bytes32,bytes)"))
        let magic_value = FixedBytes([0x16, 0x26, 0xba, 0x7e]);
        assert_eq!(magic_value.as_slice(), &[0x16, 0x26, 0xba, 0x7e]);
    }
}
