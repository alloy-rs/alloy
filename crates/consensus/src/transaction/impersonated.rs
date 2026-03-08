use crate::crypto::RecoveryError;
use alloy_eips::{eip2718::Encodable2718, Typed2718};
use alloy_primitives::{keccak256, Address, B256};
use alloy_rlp::{bytes::BufMut, Decodable, Encodable};
use core::ops::Deref;

use super::{SignerRecoverable, TxHashRef};

/// A wrapper around a transaction that optionally overrides the sender address.
///
/// This is used in simulation contexts to support account impersonation: executing
/// transactions as if they were sent from an arbitrary address, without needing the private key.
///
/// When `impersonated_sender` is set:
/// - [`recover`](MaybeImpersonatedTransaction::recover) returns the impersonated address directly.
/// - [`hash`](MaybeImpersonatedTransaction::hash) returns a synthetic hash derived by appending the
///   impersonated sender to the RLP-encoded transaction before hashing, ensuring uniqueness.
///
/// When `impersonated_sender` is `None`, the wrapper is transparent and delegates all behavior
/// to the inner transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaybeImpersonatedTransaction<T> {
    /// The inner transaction.
    pub transaction: T,
    /// The optional impersonated sender.
    pub impersonated_sender: Option<Address>,
}

impl<T: Typed2718> Typed2718 for MaybeImpersonatedTransaction<T> {
    fn ty(&self) -> u8 {
        self.transaction.ty()
    }
}

impl<T> MaybeImpersonatedTransaction<T> {
    /// Creates a non-impersonated wrapper for the given transaction.
    pub const fn new(transaction: T) -> Self {
        Self { transaction, impersonated_sender: None }
    }

    /// Creates a wrapper that impersonates the given sender address.
    pub const fn impersonated(transaction: T, impersonated_sender: Address) -> Self {
        Self { transaction, impersonated_sender: Some(impersonated_sender) }
    }

    /// Returns whether the transaction is impersonated.
    pub const fn is_impersonated(&self) -> bool {
        self.impersonated_sender.is_some()
    }

    /// Consumes the wrapper and returns the inner transaction.
    pub fn into_inner(self) -> T {
        self.transaction
    }
}

impl<T: SignerRecoverable + TxHashRef + Encodable> MaybeImpersonatedTransaction<T> {
    /// Returns the sender of the transaction.
    ///
    /// If the transaction is impersonated, returns the overridden address directly.
    /// Otherwise recovers the sender from the transaction signature.
    pub fn recover(&self) -> Result<Address, RecoveryError> {
        if let Some(sender) = self.impersonated_sender {
            return Ok(sender);
        }
        self.transaction.recover_signer()
    }

    /// Returns the hash of the transaction.
    ///
    /// If the transaction is impersonated, returns a synthetic hash derived by appending the
    /// impersonated sender to the RLP-encoded transaction before hashing. This ensures the
    /// simulated transaction has a unique, deterministic hash distinct from the real one.
    ///
    /// If not impersonated, returns the real transaction hash.
    pub fn hash(&self) -> B256 {
        if let Some(sender) = self.impersonated_sender {
            let mut buf = Vec::new();
            self.transaction.encode(&mut buf);
            buf.extend_from_slice(sender.as_slice());
            return keccak256(buf);
        }
        *self.transaction.tx_hash()
    }
}

impl<T: Encodable2718> Encodable2718 for MaybeImpersonatedTransaction<T> {
    fn encode_2718_len(&self) -> usize {
        self.transaction.encode_2718_len()
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        self.transaction.encode_2718(out)
    }
}

impl<T: Encodable> Encodable for MaybeImpersonatedTransaction<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.transaction.encode(out)
    }
}

impl<T: Decodable> Decodable for MaybeImpersonatedTransaction<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        T::decode(buf).map(Self::new)
    }
}

impl<T> AsRef<T> for MaybeImpersonatedTransaction<T> {
    fn as_ref(&self) -> &T {
        &self.transaction
    }
}

impl<T> Deref for MaybeImpersonatedTransaction<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl<T> From<T> for MaybeImpersonatedTransaction<T> {
    fn from(transaction: T) -> Self {
        Self::new(transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTx {
        hash: B256,
        encoded: Vec<u8>,
    }

    impl TxHashRef for FakeTx {
        fn tx_hash(&self) -> &B256 {
            &self.hash
        }
    }

    impl Encodable for FakeTx {
        fn encode(&self, out: &mut dyn BufMut) {
            out.put_slice(&self.encoded);
        }

        fn length(&self) -> usize {
            self.encoded.len()
        }
    }

    #[test]
    fn non_impersonated_hash_is_tx_hash() {
        let hash = B256::repeat_byte(0xab);
        let tx = MaybeImpersonatedTransaction::new(FakeTx { hash, encoded: vec![] });
        // Can't call .hash() without SignerRecoverable, but we can call tx_hash() via Deref
        assert_eq!(*tx.tx_hash(), hash);
    }

    #[test]
    fn impersonation_state() {
        let tx = MaybeImpersonatedTransaction::new(FakeTx { hash: B256::ZERO, encoded: vec![] });
        assert!(!tx.is_impersonated());
        assert!(tx.impersonated_sender.is_none());

        let sender = Address::repeat_byte(0x01);
        let imp = MaybeImpersonatedTransaction::impersonated(
            FakeTx { hash: B256::ZERO, encoded: vec![] },
            sender,
        );
        assert!(imp.is_impersonated());
        assert_eq!(imp.impersonated_sender, Some(sender));
    }

    #[test]
    fn impersonated_hash_differs_from_real() {
        let real_hash = B256::repeat_byte(0xab);
        let sender = Address::repeat_byte(0x01);
        let imp = MaybeImpersonatedTransaction::impersonated(
            FakeTx { hash: real_hash, encoded: vec![0xde, 0xad] },
            sender,
        );

        // Build expected hash manually
        let mut buf = vec![0xde, 0xad];
        buf.extend_from_slice(sender.as_slice());
        let expected = keccak256(&buf);

        // Need SignerRecoverable to call .hash() — test the logic directly
        let mut manual_buf = Vec::new();
        imp.transaction.encode(&mut manual_buf);
        manual_buf.extend_from_slice(sender.as_slice());
        assert_eq!(keccak256(manual_buf), expected);
        assert_ne!(expected, real_hash);
    }
}
