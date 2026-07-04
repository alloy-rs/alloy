//! [EIP-7928] Block-Level Access Lists types.
//!
//! This module re-exports types from the [`alloy-eip7928`] crate.
//!
//! [EIP-7928]: https://eips.ethereum.org/EIPS/eip-7928
//! [`alloy-eip7928`]: https://crates.io/crates/alloy-eip7928

pub use alloy_eip7928::*;

#[cfg(all(test, feature = "borsh"))]
mod tests {
    use super::bal::Bal;

    fn assert_borsh<T: borsh::BorshSerialize + borsh::BorshDeserialize>() {}

    #[test]
    fn bal_has_borsh_when_feature_enabled() {
        assert_borsh::<Bal>();
    }
}
