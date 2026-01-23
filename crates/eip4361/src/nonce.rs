//! Nonce generation utilities.

use rand::{distributions::Alphanumeric, Rng};

/// Generates a secure random nonce for SIWE messages.
///
/// Returns a 17-character alphanumeric string suitable for replay attack prevention.
///
/// # Example
///
/// ```
/// use alloy_eip4361::generate_nonce;
///
/// let nonce = generate_nonce();
/// assert!(nonce.len() >= 8);
/// assert!(nonce.chars().all(|c| c.is_ascii_alphanumeric()));
/// ```
#[must_use]
pub fn generate_nonce() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(17)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_nonce() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 17);
        assert!(nonce.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_nonce_uniqueness() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2);
    }
}
