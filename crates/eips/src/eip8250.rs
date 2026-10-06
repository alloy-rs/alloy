//! EIP-8250 keyed nonce domains.

use alloc::vec::Vec;
use alloy_primitives::{address, keccak256, Address, Keccak256, B256, U256};
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
    let mut hasher = Keccak256::new();
    hasher.update(U256::from(keys.len()).to_be_bytes::<32>());
    for key in keys {
        hasher.update(key.to_be_bytes::<32>());
    }
    hasher.finalize()
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
            vec![],
            vec![U256::ZERO, U256::from(1)],
            vec![U256::from(1), U256::from(1)],
            vec![U256::from(2), U256::from(1)],
            vec![U256::from(1), U256::ZERO],
            (1..=17).map(U256::from).collect(),
        ] {
            assert!(validate_nonce_keys(&keys).is_err());
        }
    }

    #[test]
    fn nonce_storage_and_key_hash_vectors() {
        assert_eq!(
            nonce_slot(address!("1000000000000000000000000000000000000001"), U256::from(1)),
            U256::from_be_bytes(
                alloy_primitives::b256!(
                    "37ec0b1050d5648531786bee10ede961dc48178860178bce6a4d18fd11678018"
                )
                .0
            )
        );
        assert_eq!(
            nonce_keys_hash(&[U256::from(1)]),
            alloy_primitives::b256!(
                "cc69885fda6bcc1a4ace058b4a62bf5e179ea78fd58a1ccd71c22cc9b688792f"
            )
        );
    }

    #[test]
    fn nonce_keys_hash_matches_concatenated_input() {
        // Include empty and oversized sets: hashing does not validate nonce keys.
        let keys: [U256; MAX_NONCE_KEYS + 1] = core::array::from_fn(|i| U256::MAX - U256::from(i));
        for len in 0..=keys.len() {
            let mut input = [0u8; 32 * (MAX_NONCE_KEYS + 2)];
            input[..32].copy_from_slice(&U256::from(len).to_be_bytes::<32>());
            for (i, key) in keys[..len].iter().enumerate() {
                input[32 * (i + 1)..32 * (i + 2)].copy_from_slice(&key.to_be_bytes::<32>());
            }
            assert_eq!(nonce_keys_hash(&keys[..len]), keccak256(&input[..32 * (len + 1)]));
        }
        let mut zero_key_input = [0u8; 64];
        zero_key_input[31] = 1;
        assert_eq!(nonce_keys_hash(&[U256::ZERO]), keccak256(zero_key_input));
    }

    #[test]
    fn canonical_nonce_calldata() {
        assert_eq!(nonce_calldata(&[U256::ZERO], 0), [0xc1, 0x80, 0x80]);
        assert_eq!(
            nonce_calldata(&[U256::from(1), U256::from(128)], 128),
            [0xc3, 0x01, 0x81, 0x80, 0x81, 0x80]
        );
    }
}
