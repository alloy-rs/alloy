use alloy_primitives::{Address, Keccak256, B256};
use std::borrow::Cow;

/// Error returned by [`dns_encode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsEncodeError {
    /// A label in the name is empty (e.g., consecutive dots or leading/trailing dot).
    EmptyLabel,
    /// A label exceeds the 63-byte maximum DNS length.
    LabelTooLong,
}

impl std::fmt::Display for DnsEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLabel => f.write_str("ENS name contains an empty label"),
            Self::LabelTooLong => f.write_str("ENS name label exceeds 63 bytes"),
        }
    }
}

impl std::error::Error for DnsEncodeError {}

/// DNS wire-format encodes an ENS name for use with the Universal Resolver's `resolve()`.
///
/// Each label is prefixed with its byte length and the sequence is terminated with `\x00`.
/// For example, `"foo.eth"` encodes to `[3, 'f', 'o', 'o', 3, 'e', 't', 'h', 0]`.
///
/// Returns an error if any label is empty or exceeds 63 bytes.
pub fn dns_encode(name: &str) -> Result<Vec<u8>, DnsEncodeError> {
    if name.is_empty() {
        return Ok(vec![0]);
    }

    // Strip variation selector as in namehash.
    const VARIATION_SELECTOR: char = '\u{fe0f}';
    let name = if name.contains(VARIATION_SELECTOR) {
        Cow::Owned(name.replace(VARIATION_SELECTOR, ""))
    } else {
        Cow::Borrowed(name)
    };

    let mut out = Vec::with_capacity(name.len() + 2);
    for label in name.split('.') {
        let bytes = label.as_bytes();
        if bytes.is_empty() {
            return Err(DnsEncodeError::EmptyLabel);
        }
        if bytes.len() > 63 {
            return Err(DnsEncodeError::LabelTooLong);
        }
        out.push(bytes.len() as u8);
        out.extend_from_slice(bytes);
    }
    out.push(0);
    Ok(out)
}

/// Returns the ENS namehash as specified in [EIP-137](https://eips.ethereum.org/EIPS/eip-137).
pub fn namehash(name: &str) -> B256 {
    if name.is_empty() {
        return B256::ZERO;
    }

    // Remove the variation selector `U+FE0F` if present.
    const VARIATION_SELECTOR: char = '\u{fe0f}';
    let name = if name.contains(VARIATION_SELECTOR) {
        Cow::Owned(name.replace(VARIATION_SELECTOR, ""))
    } else {
        Cow::Borrowed(name)
    };

    // Generate the node starting from the right.
    // This buffer is `[node @ [u8; 32], label_hash @ [u8; 32]]`.
    let mut buffer = [0u8; 64];
    for label in name.rsplit('.') {
        // node = keccak256([node, keccak256(label)])

        // Hash the label.
        let mut label_hasher = Keccak256::new();
        label_hasher.update(label.as_bytes());
        label_hasher.finalize_into(&mut buffer[32..]);

        // Hash both the node and the label hash, writing into the node.
        let mut buffer_hasher = Keccak256::new();
        buffer_hasher.update(buffer.as_slice());
        buffer_hasher.finalize_into(&mut buffer[..32]);
    }
    buffer[..32].try_into().unwrap()
}

/// Returns the reverse-registrar name of an address.
pub fn reverse_address(addr: &Address) -> String {
    format!("{addr:x}.addr.reverse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;

    fn assert_hex(hash: B256, val: &str) {
        assert_eq!(hash.0[..], hex::decode(val).unwrap()[..]);
    }

    #[test]
    fn test_namehash() {
        for (name, expected) in &[
            ("", "0x0000000000000000000000000000000000000000000000000000000000000000"),
            ("eth", "0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"),
            ("foo.eth", "0xde9b09fd7c5f901e23a3f19fecc54828e9c848539801e86591bd9801b019f84f"),
            ("alice.eth", "0x787192fc5378cc32aa956ddfdedbf26b24e8d78e40109add0eea2c1a012c3dec"),
            ("ret↩️rn.eth", "0x3de5f4c02db61b221e7de7f1c40e29b6e2f07eb48d65bf7e304715cd9ed33b24"),
        ] {
            assert_hex(namehash(name), expected);
        }
    }

    #[test]
    fn test_reverse_address() {
        for (addr, expected) in [
            (
                "0x314159265dd8dbb310642f98f50c066173c1259b",
                "314159265dd8dbb310642f98f50c066173c1259b.addr.reverse",
            ),
            (
                "0x28679A1a632125fbBf7A68d850E50623194A709E",
                "28679a1a632125fbbf7a68d850e50623194a709e.addr.reverse",
            ),
        ] {
            assert_eq!(reverse_address(&addr.parse().unwrap()), expected, "{addr}");
        }
    }

    #[test]
    fn test_dns_encode() {
        assert_eq!(dns_encode("").unwrap(), vec![0]);
        assert_eq!(
            dns_encode("foo.eth").unwrap(),
            vec![3, b'f', b'o', b'o', 3, b'e', b't', b'h', 0]
        );
        assert_eq!(
            dns_encode("alice.eth").unwrap(),
            vec![5, b'a', b'l', b'i', b'c', b'e', 3, b'e', b't', b'h', 0]
        );
        // Known vector: "integration-tests.eth"
        assert_eq!(
            dns_encode("integration-tests.eth").unwrap(),
            hex::decode("11696e746567726174696f6e2d74657374730365746800").unwrap()
        );
        // Empty label errors
        assert_eq!(dns_encode(".eth").unwrap_err(), DnsEncodeError::EmptyLabel);
        assert_eq!(dns_encode("foo..eth").unwrap_err(), DnsEncodeError::EmptyLabel);
        // Label too long
        let long_label = "a".repeat(64) + ".eth";
        assert_eq!(dns_encode(&long_label).unwrap_err(), DnsEncodeError::LabelTooLong);
    }
}
