use crate::ENS_REVERSE_REGISTRAR_DOMAIN;
use alloy_primitives::{Address, Keccak256, B256};
use std::borrow::Cow;

/// Error returned by [`try_dns_encode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsEncodeError {
    /// A label in the name is empty, e.g. because of consecutive, leading, or trailing dots.
    EmptyLabel,
    /// A label exceeds the 255-byte maximum of the ENS DNS wire encoding.
    LabelTooLong,
}

impl std::fmt::Display for DnsEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLabel => f.write_str("ENS name contains an empty label"),
            Self::LabelTooLong => f.write_str("ENS name label exceeds 255 bytes"),
        }
    }
}

impl std::error::Error for DnsEncodeError {}

/// Returns the ENS namehash as specified in [EIP-137](https://eips.ethereum.org/EIPS/eip-137).
///
/// `name` must already be ENSIP-15 normalized. Apart from removing the `U+FE0F` variation selector,
/// this function hashes labels verbatim and does not normalize or validate them.
pub fn namehash(name: &str) -> B256 {
    if name.is_empty() {
        return B256::ZERO;
    }

    let name = strip_variation_selector(name);

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

/// Encodes a domain name into DNS wire format as specified in
/// [RFC 1035](https://datatracker.ietf.org/doc/html/rfc1035).
///
/// Each label is prefixed with its length byte, and the name is terminated with a
/// zero-length label (null byte).
///
/// This is an unchecked encoder: `name` must already be ENSIP-15 normalized and non-empty, must
/// not contain empty labels (including leading, trailing, or repeated dots), and each UTF-8 label
/// length must fit in a `u8`. Invalid input is encoded without an error, and an oversized label's
/// length prefix is silently truncated, producing corrupted wire data. Use [`try_dns_encode`] to
/// validate the name instead.
///
/// # Examples
///
/// ```
/// use alloy_ens::dns_encode;
/// assert_eq!(dns_encode("eth"), vec![3, b'e', b't', b'h', 0]);
/// assert_eq!(
///     dns_encode("vitalik.eth"),
///     vec![7, b'v', b'i', b't', b'a', b'l', b'i', b'k', 3, b'e', b't', b'h', 0]
/// );
/// ```
pub fn dns_encode(name: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(name.len() + 2);
    for label in name.split('.') {
        result.push(label.len() as u8);
        result.extend_from_slice(label.as_bytes());
    }
    result.push(0);
    result
}

/// Encodes an ENS name into DNS wire format for the Universal Resolver, validating its labels.
///
/// Like [`dns_encode`], each label is prefixed with its byte length and the name is terminated
/// with a null byte, but the `U+FE0F` variation selector is removed first so that the encoded name
/// hashes to [`namehash`] on chain. Returns an error if any label is empty or exceeds 255 bytes,
/// matching the ENS [`NameCoder`] limits.
///
/// The empty name encodes to the root (`[0]`).
///
/// [`NameCoder`]: https://github.com/ensdomains/ens-contracts/blob/staging/contracts/utils/NameCoder.sol
pub fn try_dns_encode(name: &str) -> Result<Vec<u8>, DnsEncodeError> {
    if name.is_empty() {
        return Ok(vec![0]);
    }

    let name = strip_variation_selector(name);
    let mut out = Vec::with_capacity(name.len() + 2);
    for label in name.split('.') {
        let label = label.as_bytes();
        if label.is_empty() {
            return Err(DnsEncodeError::EmptyLabel);
        }
        let Ok(len) = u8::try_from(label.len()) else {
            return Err(DnsEncodeError::LabelTooLong);
        };
        out.push(len);
        out.extend_from_slice(label);
    }
    out.push(0);
    Ok(out)
}

/// Returns the reverse-registrar name of an address.
pub fn reverse_address(addr: &Address) -> String {
    format!("{addr:x}.{ENS_REVERSE_REGISTRAR_DOMAIN}")
}

/// Removes the `U+FE0F` variation selector, which ENS ignores when hashing names.
fn strip_variation_selector(name: &str) -> Cow<'_, str> {
    const VARIATION_SELECTOR: char = '\u{fe0f}';
    if name.contains(VARIATION_SELECTOR) {
        Cow::Owned(name.replace(VARIATION_SELECTOR, ""))
    } else {
        Cow::Borrowed(name)
    }
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
    fn test_try_dns_encode() {
        assert_eq!(try_dns_encode(""), Ok(vec![0]));
        assert_eq!(try_dns_encode("foo.eth"), Ok(dns_encode("foo.eth")));
        assert_eq!(
            try_dns_encode("integration-tests.eth"),
            Ok(hex::decode("11696e746567726174696f6e2d74657374730365746800").unwrap())
        );
        assert_eq!(try_dns_encode("ret↩️rn.eth"), Ok(dns_encode("ret↩rn.eth")));

        assert_eq!(try_dns_encode(".eth"), Err(DnsEncodeError::EmptyLabel));
        assert_eq!(try_dns_encode("foo..eth"), Err(DnsEncodeError::EmptyLabel));
        assert_eq!(try_dns_encode("foo.eth."), Err(DnsEncodeError::EmptyLabel));

        let max_label = "a".repeat(255) + ".eth";
        assert!(try_dns_encode(&max_label).is_ok());
        let too_long = "a".repeat(256) + ".eth";
        assert_eq!(try_dns_encode(&too_long), Err(DnsEncodeError::LabelTooLong));
    }
}
