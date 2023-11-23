use crate::utils::public_key_to_address;
use alloy_primitives::{keccak256, Address, B256};
use elliptic_curve::NonZeroScalar;
use k256::{
    ecdsa::{self, RecoveryId, VerifyingKey},
    Secp256k1,
};

/// An Ethereum ECDSA signature.
///
/// This is a wrapper around [`ecdsa::Signature`] and a [`RecoveryId`] to provide public key
/// recovery functionality.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(missing_copy_implementations)]
pub struct Signature {
    /// The inner ECDSA signature.
    inner: ecdsa::Signature,
    /// The recovery ID.
    recid: RecoveryId,
}

impl Signature {
    /// Creates a new signature from the given inner signature and recovery ID.
    pub const fn new(inner: ecdsa::Signature, recid: RecoveryId) -> Self {
        Self { inner, recid }
    }

    /// Parses a signature from a byte slice.
    #[inline]
    pub fn from_bytes(bytes: &[u8], v: u64) -> Result<Self, ecdsa::Error> {
        let inner = ecdsa::Signature::from_slice(bytes)?;
        let recid = normalize_v(v);
        Ok(Self { inner, recid })
    }

    /// Creates a [`Signature`] from the serialized `r` and `s` scalar values, which comprise the
    /// ECDSA signature, alongside a `v` value, used to determine the recovery ID.
    ///
    /// See [`ecdsa::Signature::from_scalars`] for more details.
    #[inline]
    pub fn from_scalars(r: B256, s: B256, v: u64) -> Result<Self, ecdsa::Error> {
        let inner = ecdsa::Signature::from_scalars(r.0, s.0)?;
        let recid = normalize_v(v);
        Ok(Self { inner, recid })
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub const fn inner(&self) -> &ecdsa::Signature {
        &self.inner
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut ecdsa::Signature {
        &mut self.inner
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub const fn into_inner(self) -> ecdsa::Signature {
        self.inner
    }

    /// Returns the recovery ID.
    #[inline]
    pub const fn recid(&self) -> RecoveryId {
        self.recid
    }

    #[doc(hidden)]
    #[deprecated(note = "use `Signature::recid` instead")]
    pub const fn recovery_id(&self) -> RecoveryId {
        self.recid
    }

    /// Returns the `r` component of this signature.
    #[inline]
    pub fn r(&self) -> NonZeroScalar<Secp256k1> {
        self.inner.r()
    }

    /// Returns the `s` component of this signature.
    #[inline]
    pub fn s(&self) -> NonZeroScalar<Secp256k1> {
        self.inner.s()
    }

    /// Returns the recovery ID as a `u8`.
    #[inline]
    pub const fn v(&self) -> u8 {
        self.recid.to_byte()
    }

    /// Sets the recovery ID.
    #[inline]
    pub fn set_recid(&mut self, recid: RecoveryId) {
        self.recid = recid;
    }

    /// Sets the recovery ID by normalizing a `v` value.
    #[inline]
    pub fn set_v(&mut self, v: u64) {
        self.recid = normalize_v(v);
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given message by first hashing the
    /// message with Keccak-256.
    #[inline]
    pub fn recover_address_from_msg<T: AsRef<[u8]>>(
        &self,
        msg: T,
    ) -> Result<Address, ecdsa::Error> {
        self.recover_from_msg(msg).map(|pubkey| public_key_to_address(&pubkey))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given prehashed message.
    #[inline]
    pub fn recover_address_from_prehash(&self, prehash: &B256) -> Result<Address, ecdsa::Error> {
        self.recover_from_prehash(prehash).map(|pubkey| public_key_to_address(&pubkey))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given message by first hashing the
    /// message with Keccak-256.
    #[inline]
    pub fn recover_from_msg<T: AsRef<[u8]>>(&self, msg: T) -> Result<VerifyingKey, ecdsa::Error> {
        self.recover_from_prehash(&keccak256(msg))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given prehashed message.
    #[inline]
    pub fn recover_from_prehash(&self, prehash: &B256) -> Result<VerifyingKey, ecdsa::Error> {
        VerifyingKey::recover_from_prehash(prehash.as_slice(), &self.inner, self.recid)
    }
}

/// Normalizes a `v` value, respecting raw, legacy, and EIP-155 values.
///
/// This function covers the entire u64 range, producing v-values as follows:
/// - 0-26 - raw/bare. 0-3 are legal. In order to ensure that all values are covered, we also handle
///   4-26 here by returning v % 4.
/// - 27-34 - legacy. 27-30 are legal. By legacy bitcoin convention range 27-30 signals uncompressed
///   pubkeys, while 31-34 signals compressed pubkeys. We do not respect the compression convention.
///   All Ethereum keys are uncompressed.
/// - 35+ - EIP-155. By EIP-155 convention, `v = 35 + CHAIN_ID * 2 + 0/1` We return (v-1 % 2) here.
///
/// NB: raw and legacy support values 2, and 3, while EIP-155 does not.
/// Recovery values of 2 and 3 are unlikely to occur in practice. In the vanishingly unlikely event
/// that you encounter an EIP-155 signature with a recovery value of 2 or 3, you should normalize
/// out of band.
#[inline]
const fn normalize_v(v: u64) -> RecoveryId {
    let byte = match v {
        // Case 0: raw/bare
        v @ 0..=26 => (v % 4) as u8,
        // Case 2: non-eip155 v value
        v @ 27..=34 => ((v - 27) % 4) as u8,
        // Case 3: eip155 V value
        v @ 35.. => ((v - 1) % 2) as u8,
    };
    match RecoveryId::from_byte(byte) {
        Some(recid) => recid,
        None => unsafe { core::hint::unreachable_unchecked() },
    }
}
