#[cfg(not(feature = "no_std"))]
use std::sync::OnceLock;

use crate::transaction::SignableTransaction;
use alloy_primitives::{Address, Signature, B256};

/// A transaction with a signature and hash seal.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signed<T, Sig = Signature> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    tx: T,
    #[cfg_attr(feature = "serde", serde(flatten))]
    signature: Sig,
    hash: B256,
    #[cfg_attr(feature = "serde", serde(skip))] // TODO: Write serde for OnceLock<Address>
    #[cfg(not(feature = "no_std"))]
    signer: OnceLock<Address>,
}

impl<T, Sig> Signed<T, Sig> {
    /// Returns a reference to the transaction.
    pub const fn tx(&self) -> &T {
        &self.tx
    }

    /// Returns a reference to the signature.
    pub const fn signature(&self) -> &Sig {
        &self.signature
    }

    /// Returns a reference to the transaction hash.
    pub const fn hash(&self) -> &B256 {
        &self.hash
    }

    /// Splits the transaction into parts.
    pub fn into_parts(self) -> (T, Sig, B256) {
        (self.tx, self.signature, self.hash)
    }

    /// Returns the transaction without signature.
    pub fn strip_signature(self) -> T {
        self.tx
    }

    /// Returns the signer of the transaction.
    #[cfg(not(feature = "no_std"))]
    pub fn signer(&self) -> Option<&Address> {
        self.signer.get()
    }

    /// Sets the signer of the transaction.
    #[cfg(not(feature = "no_std"))]
    fn set_signer(&self, signer: Address) -> Result<(), Address> {
        self.signer.set(signer)
    }
}

impl<T: SignableTransaction<Sig>, Sig> Signed<T, Sig> {
    /// Instantiate from a transaction and signature. Does not verify the signature.
    pub const fn new_unchecked(tx: T, signature: Sig, hash: B256) -> Self {
        Self { tx, signature, hash, signer: OnceLock::new() }
    }

    /// Calculate the signing hash for the transaction.
    pub fn signature_hash(&self) -> B256 {
        self.tx.signature_hash()
    }
}

#[cfg(feature = "k256")]
impl<T: SignableTransaction<Signature>> Signed<T, Signature> {
    /// Recover the signer of the transaction
    pub fn recover_signer(
        &self,
    ) -> Result<alloy_primitives::Address, alloy_primitives::SignatureError> {
        #[cfg(not(feature = "no_std"))]
        if let Some(signer) = self.signer() {
            return Ok(*signer);
        }
        let sighash = self.tx.signature_hash();
        let signer = self.signature.recover_address_from_prehash(&sighash)?;

        #[cfg(not(feature = "no_std"))]
        let _ = self.set_signer(signer);

        Ok(signer)
    }
}
