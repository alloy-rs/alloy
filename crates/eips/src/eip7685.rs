//! [EIP-7685]: General purpose execution layer requests
//!
//! [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685

use alloc::vec::Vec;
use alloy_primitives::Bytes;
use derive_more::{Deref, DerefMut, From, IntoIterator};

/// A list of opaque EIP-7685 requests.
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
    pub fn take(self) -> Vec<Bytes> {
        self.0
    }

    /// Get an iterator over the Requests.
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
    #[cfg(feature = "sha2")]
    pub fn requests_hash(&self) -> alloy_primitives::B256 {
        use sha2::{Digest, Sha256};
        let mut hash = sha2::Sha256::new();
        for (ty, req) in self.0.iter().enumerate() {
            let mut req_hash = Sha256::new();
            req_hash.update([ty as u8]);
            req_hash.update(req);
            hash.update(req_hash.finalize());
        }
        alloy_primitives::B256::from(hash.finalize().as_ref())
    }

    /// Extend this container with requests from another container.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.take());
    }
}
