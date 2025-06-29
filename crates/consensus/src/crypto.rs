//! Cryptographic algorithms

use alloc::boxed::Box;
use alloy_primitives::U256;

#[cfg(any(feature = "secp256k1", feature = "k256"))]
use alloy_primitives::Signature;

/// Opaque error type for sender recovery.
#[derive(Debug, Default, thiserror::Error)]
#[error("Failed to recover the signer")]
pub struct RecoveryError {
    #[source]
    source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
}

impl RecoveryError {
    /// Create a new error with no associated source
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new error with an associated source.
    ///
    /// **NOTE:** The "source" should **NOT** be used to propagate cryptographic
    /// errors e.g. signature parsing or verification errors.
    pub fn from_source<E: core::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self { source: Some(Box::new(err)) }
    }
}

impl From<alloy_primitives::SignatureError> for RecoveryError {
    fn from(err: alloy_primitives::SignatureError) -> Self {
        Self::from_source(err)
    }
}

/// The order of the secp256k1 curve, divided by two. Signatures that should be checked according
/// to EIP-2 should have an S value less than or equal to this.
///
/// `57896044618658097711785492504343953926418782139537452191302581570759080747168`
pub const SECP256K1N_HALF: U256 = U256::from_be_bytes([
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B, 0x20, 0xA0,
]);

/// Crypto backend module for pluggable cryptographic implementations.
#[cfg(feature = "crypto-backend")]
pub mod backend {
    use super::*;
    use alloc::sync::Arc;
    use alloy_primitives::{Address, Signature, B256};
    use core::sync::atomic::{AtomicBool, Ordering};

    #[cfg(feature = "std")]
    use std::sync::OnceLock;

    #[cfg(not(feature = "std"))]
    use once_cell::sync::OnceBox;

    /// Trait for cryptographic providers that can perform signature recovery.
    pub trait CryptoProvider: Send + Sync + 'static {
        /// Error type for this provider.
        type Error: core::error::Error + Send + Sync + 'static;

        /// Recover signer from signature and message hash, without ensuring low S values.
        fn recover_signer_unchecked(
            &self,
            sig: &[u8; 65],
            msg: &[u8; 32],
        ) -> Result<Address, Self::Error>;
    }

    /// Global default crypto provider.
    #[cfg(feature = "std")]
    static DEFAULT_PROVIDER: OnceLock<Arc<dyn CryptoProvider<Error = RecoveryError>>> =
        OnceLock::new();

    #[cfg(not(feature = "std"))]
    static DEFAULT_PROVIDER: OnceBox<Arc<dyn CryptoProvider<Error = RecoveryError>>> =
        OnceBox::new();

    /// Error returned when attempting to install a provider when one is already installed.
    /// Contains the provider that was attempted to be installed.
    #[derive(Debug)]
    pub struct ProviderAlreadySetError {
        pub provider: Arc<dyn CryptoProvider<Error = RecoveryError>>,
    }

    impl core::fmt::Display for ProviderAlreadySetError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "crypto provider already installed")
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for ProviderAlreadySetError {}

    /// Install the default crypto provider.
    ///
    /// This sets the global default provider used by the high-level crypto functions.
    /// Returns an error containing the provider that was attempted to be installed if one is
    /// already set.
    pub fn install_default_provider(
        provider: Arc<dyn CryptoProvider<Error = RecoveryError>>,
    ) -> Result<(), ProviderAlreadySetError> {
        #[cfg(feature = "std")]
        {
            DEFAULT_PROVIDER.set(provider.clone()).map_err(|_| {
                // Return the provider we tried to install in the error
                ProviderAlreadySetError { provider }
            })
        }
        #[cfg(not(feature = "std"))]
        {
            DEFAULT_PROVIDER.set(Box::new(provider.clone())).map_err(|_| {
                // Return the provider we tried to install in the error
                ProviderAlreadySetError { provider }
            })
        }
    }

    /// Get the currently installed default provider, panicking if none is installed.
    pub(super) fn get_default_provider() -> &'static dyn CryptoProvider<Error = RecoveryError> {
        match try_get_provider() {
            Some(provider) => provider,
            None => panic!("No crypto backend installed. Call install_default_provider() first."),
        }
    }

    /// Try to get the currently installed default provider, returning None if none is installed.
    pub(super) fn try_get_provider() -> Option<&'static dyn CryptoProvider<Error = RecoveryError>> {
        #[cfg(feature = "std")]
        {
            DEFAULT_PROVIDER.get().map(|arc| arc.as_ref())
        }
        #[cfg(not(feature = "std"))]
        {
            DEFAULT_PROVIDER.get().map(|boxed_arc| boxed_arc.as_ref().as_ref())
        }
    }
}

/// Secp256k1 cryptographic functions.
#[cfg(any(feature = "secp256k1", feature = "k256"))]
pub mod secp256k1 {
    pub use imp::{public_key_to_address, sign_message};

    use super::*;
    use alloy_primitives::{Address, B256};

    #[cfg(not(feature = "secp256k1"))]
    use super::impl_k256 as imp;
    #[cfg(feature = "secp256k1")]
    use super::impl_secp256k1 as imp;

    /// Recover signer from message hash, _without ensuring that the signature has a low `s`
    /// value_.
    ///
    /// Using this for signature validation will succeed, even if the signature is malleable or not
    /// compliant with EIP-2. This is provided for compatibility with old signatures which have
    /// large `s` values.
    pub fn recover_signer_unchecked(
        signature: &Signature,
        hash: B256,
    ) -> Result<Address, RecoveryError> {
        let mut sig: [u8; 65] = [0; 65];

        sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
        sig[64] = signature.v() as u8;

        // Try dynamic backend first when crypto-backend feature is enabled
        #[cfg(feature = "crypto-backend")]
        if let Some(provider) = super::backend::try_get_provider() {
            return provider.recover_signer_unchecked(&sig, &hash.0);
        }

        // Fallback to compile-time selected implementation
        // NOTE: we are removing error from underlying crypto library as it will restrain primitive
        // errors and we care only if recovery is passing or not.
        imp::recover_signer_unchecked(&sig, &hash.0).map_err(|_| RecoveryError::new())
    }

    /// Recover signer address from message hash. This ensures that the signature S value is
    /// lower than `secp256k1n / 2`, as specified in
    /// [EIP-2](https://eips.ethereum.org/EIPS/eip-2).
    ///
    /// If the S value is too large, then this will return a `RecoveryError`
    pub fn recover_signer(signature: &Signature, hash: B256) -> Result<Address, RecoveryError> {
        if signature.s() > SECP256K1N_HALF {
            return Err(RecoveryError::new());
        }
        recover_signer_unchecked(signature, hash)
    }
}

#[cfg(any(test, feature = "secp256k1"))]
mod impl_secp256k1 {
    pub(crate) use ::secp256k1::Error;
    use ::secp256k1::{
        ecdsa::{RecoverableSignature, RecoveryId},
        Message, PublicKey, SecretKey, SECP256K1,
    };
    use alloy_primitives::{keccak256, Address, Signature, B256, U256};

    /// Recovers the address of the sender using secp256k1 pubkey recovery.
    ///
    /// Converts the public key into an ethereum address by hashing the public key with keccak256.
    ///
    /// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
    /// underlying secp256k1 library.
    pub(crate) fn recover_signer_unchecked(
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, Error> {
        let sig =
            RecoverableSignature::from_compact(&sig[0..64], RecoveryId::try_from(sig[64] as i32)?)?;

        let public = SECP256K1.recover_ecdsa(&Message::from_digest(*msg), &sig)?;
        Ok(public_key_to_address(public))
    }

    /// Signs message with the given secret key.
    /// Returns the corresponding signature.
    pub fn sign_message(secret: B256, message: B256) -> Result<Signature, Error> {
        let sec = SecretKey::from_slice(secret.as_ref())?;
        let s = SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(message.0), &sec);
        let (rec_id, data) = s.serialize_compact();

        let signature = Signature::new(
            U256::try_from_be_slice(&data[..32]).expect("The slice has at most 32 bytes"),
            U256::try_from_be_slice(&data[32..64]).expect("The slice has at most 32 bytes"),
            i32::from(rec_id) != 0,
        );
        Ok(signature)
    }

    /// Converts a public key into an ethereum address by hashing the encoded public key with
    /// keccak256.
    pub fn public_key_to_address(public: PublicKey) -> Address {
        // strip out the first byte because that should be the SECP256K1_TAG_PUBKEY_UNCOMPRESSED
        // tag returned by libsecp's uncompressed pubkey serialization
        let hash = keccak256(&public.serialize_uncompressed()[1..]);
        Address::from_slice(&hash[12..])
    }
}

#[cfg(feature = "k256")]
#[cfg_attr(feature = "secp256k1", allow(unused, unreachable_pub))]
mod impl_k256 {
    pub(crate) use k256::ecdsa::Error;

    use super::*;
    use alloy_primitives::{keccak256, Address, B256};
    use k256::ecdsa::{RecoveryId, SigningKey, VerifyingKey};

    /// Recovers the address of the sender using secp256k1 pubkey recovery.
    ///
    /// Converts the public key into an ethereum address by hashing the public key with keccak256.
    ///
    /// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
    /// underlying secp256k1 library.
    pub(crate) fn recover_signer_unchecked(
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, Error> {
        let mut signature = k256::ecdsa::Signature::from_slice(&sig[0..64])?;
        let mut recid = sig[64];

        // normalize signature and flip recovery id if needed.
        if let Some(sig_normalized) = signature.normalize_s() {
            signature = sig_normalized;
            recid ^= 1;
        }
        let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");

        // recover key
        let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &signature, recid)?;
        Ok(public_key_to_address(recovered_key))
    }

    /// Signs message with the given secret key.
    /// Returns the corresponding signature.
    pub fn sign_message(secret: B256, message: B256) -> Result<Signature, Error> {
        let sec = SigningKey::from_slice(secret.as_ref())?;
        sec.sign_prehash_recoverable(&message.0).map(Into::into)
    }

    /// Converts a public key into an ethereum address by hashing the encoded public key with
    /// keccak256.
    pub fn public_key_to_address(public: VerifyingKey) -> Address {
        let hash = keccak256(&public.to_encoded_point(/* compress = */ false).as_bytes()[1..]);
        Address::from_slice(&hash[12..])
    }
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "secp256k1")]
    #[test]
    fn sanity_ecrecover_call_secp256k1() {
        use super::impl_secp256k1::*;
        use alloy_primitives::B256;

        let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
        let signer = public_key_to_address(public);

        let message = b"hello world";
        let hash = alloy_primitives::keccak256(message);
        let signature =
            sign_message(B256::from_slice(&secret.secret_bytes()[..]), hash).expect("sign message");

        let mut sig: [u8; 65] = [0; 65];
        sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
        sig[64] = signature.v() as u8;

        assert_eq!(recover_signer_unchecked(&sig, &hash), Ok(signer));
    }

    #[cfg(feature = "k256")]
    #[test]
    fn sanity_ecrecover_call_k256() {
        use super::impl_k256::*;
        use alloy_primitives::B256;

        let secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let public = *secret.verifying_key();
        let signer = public_key_to_address(public);

        let message = b"hello world";
        let hash = alloy_primitives::keccak256(message);
        let signature =
            sign_message(B256::from_slice(&secret.to_bytes()[..]), hash).expect("sign message");

        let mut sig: [u8; 65] = [0; 65];
        sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
        sig[64] = signature.v() as u8;

        assert_eq!(recover_signer_unchecked(&sig, &hash).ok(), Some(signer));
    }

    #[test]
    #[cfg(all(feature = "secp256k1", feature = "k256"))]
    fn sanity_secp256k1_k256_compat() {
        use super::{impl_k256, impl_secp256k1};
        use alloy_primitives::B256;

        let (secp256k1_secret, secp256k1_public) =
            secp256k1::generate_keypair(&mut rand::thread_rng());
        let k256_secret = k256::ecdsa::SigningKey::from_slice(&secp256k1_secret.secret_bytes())
            .expect("k256 secret");
        let k256_public = *k256_secret.verifying_key();

        let secp256k1_signer = impl_secp256k1::public_key_to_address(secp256k1_public);
        let k256_signer = impl_k256::public_key_to_address(k256_public);
        assert_eq!(secp256k1_signer, k256_signer);

        let message = b"hello world";
        let hash = alloy_primitives::keccak256(message);

        let secp256k1_signature = impl_secp256k1::sign_message(
            B256::from_slice(&secp256k1_secret.secret_bytes()[..]),
            hash,
        )
        .expect("secp256k1 sign");
        let k256_signature =
            impl_k256::sign_message(B256::from_slice(&k256_secret.to_bytes()[..]), hash)
                .expect("k256 sign");
        assert_eq!(secp256k1_signature, k256_signature);

        let mut sig: [u8; 65] = [0; 65];

        sig[0..32].copy_from_slice(&secp256k1_signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&secp256k1_signature.s().to_be_bytes::<32>());
        sig[64] = secp256k1_signature.v() as u8;
        let secp256k1_recovered =
            impl_secp256k1::recover_signer_unchecked(&sig, &hash).expect("secp256k1 recover");
        assert_eq!(secp256k1_recovered, secp256k1_signer);

        sig[0..32].copy_from_slice(&k256_signature.r().to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&k256_signature.s().to_be_bytes::<32>());
        sig[64] = k256_signature.v() as u8;
        let k256_recovered =
            impl_k256::recover_signer_unchecked(&sig, &hash).expect("k256 recover");
        assert_eq!(k256_recovered, k256_signer);

        assert_eq!(secp256k1_recovered, k256_recovered);
    }

    #[cfg(feature = "crypto-backend")]
    mod backend_tests {
        use super::*;
        use crate::crypto::backend::{install_default_provider, try_get_provider, CryptoProvider};
        use alloy_primitives::{Address, Signature, B256};

        /// Mock crypto provider for testing
        struct MockCryptoProvider {
            should_fail: bool,
            return_address: Address,
        }

        impl CryptoProvider for MockCryptoProvider {
            type Error = RecoveryError;

            fn recover_signer_unchecked(
                &self,
                _sig: &[u8; 65],
                _msg: &[u8; 32],
            ) -> Result<Address, Self::Error> {
                if self.should_fail {
                    Err(RecoveryError::new())
                } else {
                    Ok(self.return_address)
                }
            }
        }

        #[test]
        fn test_crypto_backend_basic_functionality() {
            // Test that when a provider is installed, it's actually used
            let custom_address = Address::from([0x99; 20]); // Unique test address
            let provider =
                Arc::new(MockCryptoProvider { should_fail: false, return_address: custom_address });

            // Try to install the provider (may fail if already set from other tests)
            let install_result = crate::crypto::backend::install_default_provider(provider);

            // Create test signature and hash
            let signature = Signature::new(
                alloy_primitives::U256::from(123u64),
                alloy_primitives::U256::from(456u64),
                false,
            );
            let hash = B256::from([0xAB; 32]);

            // Call the high-level function
            let result = crate::crypto::secp256k1::recover_signer_unchecked(&signature, hash);

            // If our provider was successfully installed, we should get our custom address
            if install_result.is_ok() {
                assert!(result.is_ok());
                assert_eq!(result.unwrap(), custom_address);
            }
            // If provider was already set, we still should get a valid result
            else {
                assert!(result.is_ok()); // Should work with any provider
            }
        }

        #[test]
        fn test_provider_already_set_error() {
            // First installation might work or fail if already set from another test
            // Since tests are ran in parallel.
            let provider1 = Arc::new(MockCryptoProvider {
                should_fail: false,
                return_address: Address::from([0x11; 20]),
            });
            let result1 = crate::crypto::backend::install_default_provider(provider1);

            // Second installation should always fail since OnceLock can only be set once
            let provider2 = Arc::new(MockCryptoProvider {
                should_fail: true,
                return_address: Address::from([0x22; 20]),
            });
            let result2 = crate::crypto::backend::install_default_provider(provider2.clone());

            // The second attempt should fail with ProviderAlreadySetError
            assert!(result2.is_err());

            // The error should contain the provider we tried to install (provider2)
            if let Err(err) = result2 {
                // Verify the returned provider is the same as the one we tried to install
                assert!(Arc::ptr_eq(&err.provider, &provider2));
            }
        }

        /// Test that when crypto-backend feature is enabled but no provider is installed,
        /// it falls back to the default implementation
        ///
        /// Note: this test is unreliable because if the CryptoProvider is set, it will use the
        /// MockProvider which doesn't have ideal behaviour
        #[test]
        #[cfg(any(feature = "secp256k1", feature = "k256"))]
        fn test_fallback_to_default_when_no_provider() {
            // This test works regardless of whether a provider is already installed
            // because it tests the fallback behavior when the provider check fails

            // Create a real signature for testing
            #[cfg(feature = "secp256k1")]
            {
                let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
                let expected_signer = crate::crypto::impl_secp256k1::public_key_to_address(public);

                let message = b"test fallback";
                let hash = alloy_primitives::keccak256(message);
                let signature = crate::crypto::impl_secp256k1::sign_message(
                    B256::from_slice(&secret.secret_bytes()[..]),
                    hash,
                )
                .expect("sign message");

                // This should always work - either via provider or fallback
                let recovered =
                    crate::crypto::secp256k1::recover_signer_unchecked(&signature, hash);
                assert!(recovered.is_ok());

                // If no provider is installed, should use fallback and match expected signer
                if try_get_provider().is_none() {
                    assert_eq!(recovered.unwrap(), expected_signer);
                }
            }
        }
    }
}
