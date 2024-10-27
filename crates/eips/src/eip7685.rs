//! [EIP-7685]: General purpose execution layer requests
//!
//! [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685

use alloc::vec::Vec;
use alloy_primitives::{b256, Bytes, B256};
use derive_more::{Deref, DerefMut, From, IntoIterator};

/// The empty requests hash.
///
/// This is equivalent to `sha256(sha256(0) ++ sha256(1) ++ sha256(2))`
pub const EMPTY_REQUESTS_HASH: B256 =
    b256!("6036c41849da9c076ed79654d434017387a88fb833c2856b32e18218b3341c5f");

/// A container of EIP-7685 requests.
///
/// The container only holds the `requests_data` as defined by their respective EIPs. The request
/// type is prepended to `requests_data` in [`Requests::requests_hash`] to calculate the requests
/// hash as definned in EIP-7685.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Deref, DerefMut, From, IntoIterator)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Requests(Vec<Bytes>);

impl Requests {
    /// Construct a new [`Requests`] container.
    pub const fn new(requests: Vec<Bytes>) -> Self {
        Self(requests)
    }

    /// Add a new request into the container.
    pub fn push_request(&mut self, request: Bytes) {
        self.0.push(request);
    }

    /// Consumes [`Requests`] and returns the inner raw opaque requests.
    ///
    /// # Note
    ///
    /// These requests are only the `requests_data` without the `request_type`.
    pub fn take(self) -> Vec<Bytes> {
        self.0
    }

    /// Get an iterator over the requests.
    pub fn iter(&self) -> core::slice::Iter<'_, Bytes> {
        self.0.iter()
    }

    /// Calculate the requests hash as defined in EIP-7685 for the requests.
    ///
    /// The requests hash is defined as
    ///
    /// ```text
    /// sha256(sha256(requests_0) ++ sha256(requests_1) ++ ...)
    /// ```
    ///
    /// The request type for each requests is prepended to the `requests_data` inside of this
    /// container. The request type for the first request in the container will be `0x00`, the
    /// second request will have type `0x01`, and so on.
    #[cfg(feature = "sha2")]
    pub fn requests_hash(&self) -> B256 {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        for (ty, req) in self.0.iter().enumerate() {
            let mut req_hash = Sha256::new();
            req_hash.update([ty as u8]);
            req_hash.update(req);
            hash.update(req_hash.finalize());
        }
        B256::from(hash.finalize().as_ref())
    }

    /// Extend this container with requests from another container.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.take());
    }
}
/// A list of requests or a precomputed requests hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PragueRequests {
    /// Stores a list of requests for dynamic requests hash calculation.
    Requests(Requests),
    /// Stores a precomputed requests hash.
    Hash(B256),
}

impl PragueRequests {
    /// Returns the requests hash for the enum instance.
    ///
    /// - If the instance contains a list of requests, this function calculates the hash using
    ///   `requests_hash` of the `Requests` struct.
    /// - If it contains a precomputed hash, it returns that hash directly.
    #[cfg(feature = "sha2")]
    pub fn requests_hash(&self) -> B256 {
        match self {
            Self::Requests(requests) => requests.requests_hash(),
            Self::Hash(precomputed_hash) => *precomputed_hash,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extend() {
        // Test extending a Requests container with another Requests container
        let mut reqs1 = Requests::new(vec![Bytes::from(vec![0x01, 0x02])]);
        let reqs2 =
            Requests::new(vec![Bytes::from(vec![0x03, 0x04]), Bytes::from(vec![0x05, 0x06])]);

        // Extend reqs1 with reqs2
        reqs1.extend(reqs2);

        // Ensure the requests are correctly combined
        assert_eq!(reqs1.0.len(), 3);
        assert_eq!(
            reqs1.0,
            vec![
                Bytes::from(vec![0x01, 0x02]),
                Bytes::from(vec![0x03, 0x04]),
                Bytes::from(vec![0x05, 0x06])
            ]
        );
    }

    #[test]
    fn test_consistent_requests_hash() {
        // We test that the empty requests hash is consistent with the EIP-7685 definition.
        assert_eq!(
            Requests(vec![Bytes::from(vec![]), Bytes::from(vec![]), Bytes::from(vec![])])
                .requests_hash(),
            EMPTY_REQUESTS_HASH,
        );

        // Test to hash a non-empty vector of requests.
        assert_eq!(
            Requests(vec![
                Bytes::from(vec![0x0a, 0x0b, 0x0c]),
                Bytes::from(vec![0x0d, 0x0e, 0x0f])
            ])
            .requests_hash(),
            b256!("be3a57667b9bb9e0275019c0faf0f415fdc8385a408fd03e13a5c50615e3530c"),
        );
    }
}
