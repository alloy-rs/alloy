use crate::Transaction;
use alloy_primitives::{Signature, B256};
use alloy_rlp::BufMut;

/// A transaction with a signature and hash seal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signed<T, Sig = Signature> {
    tx: T,
    signature: Sig,
    hash: B256,
}

impl<T> std::ops::Deref for Signed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl<T, Sig> Signed<T, Sig> {
    /// Return a reference to the transactions
    pub const fn tx(&self) -> &T {
        &self.tx
    }

    /// Get a reference to the signature
    pub const fn signature(&self) -> &Sig {
        &self.signature
    }

    /// Get the transaction hash.
    pub const fn hash(&self) -> B256 {
        self.hash
    }
}

impl<T: Transaction> Signed<T> {
    /// Instantiate from a transaction and signature. Does not verify the signature.
    pub const fn new_unchecked(tx: T, signature: Signature, hash: B256) -> Self {
        Self { tx, signature, hash }
    }

    /// Output the signed RLP for the transaction.
    pub fn encode_signed(&self, out: &mut dyn BufMut) {
        self.tx.encode_signed(&self.signature, out);
    }

    /// Produce the RLP encoded signed transaction.
    pub fn rlp_signed(&self) -> Vec<u8> {
        let mut buf = vec![];
        self.encode_signed(&mut buf);
        buf
    }
}

impl<T: Transaction> alloy_rlp::Encodable for Signed<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.tx.encode_signed(&self.signature, out)
    }

    // TODO: impl length
}

impl<T: Transaction> alloy_rlp::Decodable for Signed<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        T::decode_signed(buf)
    }
}

#[cfg(feature = "k256")]
impl<T: Transaction> Signed<T, Signature> {
    /// Recover the signer of the transaction
    pub fn recover_signer(
        &self,
    ) -> Result<alloy_primitives::Address, alloy_primitives::SignatureError> {
        let sighash = self.tx.signature_hash();
        self.signature.recover_address_from_prehash(sighash)
    }
}
