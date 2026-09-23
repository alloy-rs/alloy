//! EIP-8250 keyed nonce domains.

use alloc::vec::Vec;
use alloy_primitives::{address, keccak256, Address, B256, U256};
use alloy_rlp::Encodable;

/// Protocol-owned keyed nonce storage account.
pub const NONCE_MANAGER: Address = address!("0000000000000000000000000000000000008250");
/// Runtime that rejects ordinary calls to the nonce manager.
pub const NONCE_MANAGER_CODE: [u8; 5] = [0x60, 0x00, 0x60, 0x00, 0xfd];
/// Maximum number of nonce domains selected by one transaction.
pub const MAX_NONCE_KEYS: usize = 16;

/// Validates the canonical set of nonce keys, including the legacy singleton `[0]`.
pub fn validate_nonce_keys(keys: &[U256]) -> Result<(), &'static str> {
    if keys.is_empty() || keys.len() > MAX_NONCE_KEYS {
        return Err("EIP-8250 requires between 1 and 16 nonce keys");
    }
    if keys.len() > 1 && keys[0].is_zero() {
        return Err("EIP-8250 nonce key zero must be the only key");
    }
    if keys.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("EIP-8250 nonce keys must be strictly increasing");
    }
    Ok(())
}

/// Returns the storage slot for a sender's nonzero nonce key.
pub fn nonce_slot(sender: Address, key: U256) -> U256 {
    let mut input = [0u8; 64];
    input[12..32].copy_from_slice(sender.as_slice());
    input[32..].copy_from_slice(&key.to_be_bytes::<32>());
    U256::from_be_bytes(keccak256(input).0)
}

/// Hashes the key count followed by each key as a 32-byte big-endian integer.
pub fn nonce_keys_hash(keys: &[U256]) -> B256 {
    let mut input = Vec::with_capacity(32 * (keys.len() + 1));
    input.extend_from_slice(&U256::from(keys.len()).to_be_bytes::<32>());
    for key in keys {
        input.extend_from_slice(&key.to_be_bytes::<32>());
    }
    keccak256(input)
}

/// Encodes the nonce fields charged as transaction calldata by EIP-8250.
pub fn nonce_calldata(keys: &[U256], nonce: u64) -> Vec<u8> {
    let mut encoded = Vec::new();
    alloy_rlp::encode_list(keys, &mut encoded);
    nonce.encode(&mut encoded);
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_nonce_sets() {
        for keys in [vec![U256::ZERO], vec![U256::MAX], (1..=16).map(U256::from).collect()] {
            assert!(validate_nonce_keys(&keys).is_ok());
        }
        for keys in [
            vec![], vec![U256::ZERO, U256::from(1)], vec![U256::from(1), U256::from(1)],
            vec![U256::from(2), U256::from(1)], vec![U256::from(1), U256::ZERO],
            (1..=17).map(U256::from).collect(),
        ] {
            assert!(validate_nonce_keys(&keys).is_err());
        }
    }
}
