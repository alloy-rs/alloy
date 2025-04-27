//! Cryptographic algorithms

/// Opaque error type for sender recovery.
#[derive(Debug, Default, thiserror::Error)]
#[error("Failed to recover the signer")]
pub struct RecoveryError;

use alloy_primitives::{U256, Signature, Address, B256};

#[cfg(any(feature = "secp256k1", feature = "k256"))]
/// The order of the secp256k1 curve, divided by two. Signatures that should be checked according
/// to EIP-2 should have an S value less than or equal to this.
pub const SECP256K1N_HALF: U256 = U256::from_be_bytes([
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B, 0x20, 0xA0,
]);

/// Secp256k1 cryptographic functions.
#[cfg(any(feature = "secp256k1", feature = "k256"))]
pub mod secp256k1 {
    use super::*;
    use alloy_primitives::{keccak256, B256};

    #[cfg(not(feature = "secp256k1"))]
    use super::impl_k256 as imp;
    #[cfg(feature = "secp256k1")]
    use super::impl_secp256k1 as imp;

    /// Recover signer from message hash, without ensuring that the signature has a low `s` value.
    pub fn recover_signer_unchecked(
        signature: &Signature,
        hash: B256,
    ) -> Result<Address, RecoveryError> {
        let sig = create_signature_bytes(signature);
        imp::recover_signer_unchecked(&sig, &hash.0).map_err(|_| RecoveryError)
    }

    /// Recover signer address from message hash, ensuring the S value is valid as per EIP-2.
    pub fn recover_signer(signature: &Signature, hash: B256) -> Result<Address, RecoveryError> {
        if signature.s() > SECP256K1N_HALF {
            return Err(RecoveryError);
        }
        recover_signer_unchecked(signature, hash)
    }

    // Helper to create a byte array for the signature
    fn create_signature_bytes(signature: &Signature) -> [u8; 65] {
        let mut sig = [0; 65];
        sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
        sig[64] = signature.v() as u8;
        sig
    }
}

#[cfg(any(test, feature = "secp256k1"))]
mod impl_secp256k1 {
    use super::*;
    use secp256k1::{ecdsa::{RecoverableSignature, RecoveryId}, Message, PublicKey, SecretKey, SECP256K1};

    pub(crate) fn recover_signer_unchecked(
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, secp256k1::Error> {
        let sig = RecoverableSignature::from_compact(&sig[0..64], RecoveryId::try_from(sig[64] as i32)?)?;
        let public = SECP256K1.recover_ecdsa(&Message::from_digest(*msg), &sig)?;
        Ok(public_key_to_address(public))
    }

    pub fn sign_message(secret: B256, message: B256) -> Result<Signature, secp256k1::Error> {
        let sec = SecretKey::from_slice(secret.as_ref())?;
        let s = SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(message.0), &sec);
        let (rec_id, data) = s.serialize_compact();
        Ok(Signature::new(
            U256::try_from_be_slice(&data[..32])?,
            U256::try_from_be_slice(&data[32..64])?,
            i32::from(rec_id) != 0,
        ))
    }

    pub fn public_key_to_address(public: PublicKey) -> Address {
        let hash = keccak256(&public.serialize_uncompressed()[1..]);
        Address::from_slice(&hash[12..])
    }
}

#[cfg(feature = "k256")]
#[cfg_attr(feature = "secp256k1", allow(unused, unreachable_pub))]
mod impl_k256 {
    use super::*;
    use k256::ecdsa::{RecoveryId, SigningKey, VerifyingKey};

    pub(crate) fn recover_signer_unchecked(
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, k256::ecdsa::Error> {
        let mut signature = k256::ecdsa::Signature::from_slice(&sig[0..64])?;
        let mut recid = sig[64];

        if let Some(sig_normalized) = signature.normalize_s() {
            signature = sig_normalized;
            recid ^= 1;
        }

        let recid = RecoveryId::from_byte(recid)?;
        let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &signature, recid)?;
        Ok(public_key_to_address(recovered_key))
    }

    pub fn sign_message(secret: B256, message: B256) -> Result<Signature, k256::ecdsa::Error> {
        let sec = SigningKey::from_slice(secret.as_ref())?;
        sec.sign_prehash_recoverable(&message.0).map(Into::into)
    }

    pub fn public_key_to_address(public: VerifyingKey) -> Address {
        let hash = keccak256(&public.to_encoded_point(false).as_bytes()[1..]);
        Address::from_slice(&hash[12..])
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{keccak256, B256};

    #[cfg(feature = "secp256k1")]
    #[test]
    fn sanity_ecrecover_call_secp256k1() {
        use super::impl_secp256k1::*;
        let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
        let signer = public_key_to_address(public);
        let message = b"hello world";
        let hash = keccak256(message);
        let signature = sign_message(B256::from_slice(&secret.secret_bytes()[..]), hash).expect("sign message");

        let sig = create_signature_bytes(&signature);

        assert_eq!(recover_signer_unchecked(&sig, &hash), Ok(signer));
    }

    #[cfg(feature = "k256")]
    #[test]
    fn sanity_ecrecover_call_k256() {
        use super::impl_k256::*;
        let secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let public = *secret.verifying_key();
        let signer = public_key_to_address(public);
        let message = b"hello world";
        let hash = keccak256(message);
        let signature = sign_message(B256::from_slice(&secret.to_bytes()[..]), hash).expect("sign message");

        let sig = create_signature_bytes(&signature);

        assert_eq!(recover_signer_unchecked(&sig, &hash).ok(), Some(signer));
    }

    #[test]
    fn sanity_secp256k1_k256_compat() {
        use super::{impl_k256, impl_secp256k1::*};

        let (secp256k1_secret, secp256k1_public) = secp256k1::generate_keypair(&mut rand::thread_rng());
        let k256_secret = k256::ecdsa::SigningKey::from_slice(&secp256k1_secret.secret_bytes())
            .expect("k256 secret");
        let k256_public = *k256_secret.verifying_key();

        let secp256k1_signer = public_key_to_address(secp256k1_public);
        let k256_signer = public_key_to_address(k256_public);

        let message = b"hello world";
        let hash = keccak256(message);

        let secp256k1_signature = sign_message(B256::from_slice(&secp256k1_secret.secret_bytes()[..]), hash).expect("secp256k1 sign");
        let k256_signature = sign_message(B256::from_slice(&k256_secret.to_bytes()[..]), hash).expect("k256 sign");

        let sig = create_signature_bytes(&secp256k1_signature);
        assert_eq!(recover_signer_unchecked(&sig, &hash), Ok(secp256k1_signer));

        let sig = create_signature_bytes(&k256_signature);
        assert_eq!(recover_signer_unchecked(&sig, &hash), Ok(k256_signer));
        assert_eq!(secp256k1_signer, k256_signer);
    }
}
