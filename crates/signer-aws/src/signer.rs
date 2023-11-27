use alloy_primitives::{hex, Address, B256};
use alloy_signer::{Result, Signature, Signer};
use aws_sdk_kms::{
    error::SdkError,
    operation::{
        get_public_key::{GetPublicKeyError, GetPublicKeyOutput},
        sign::{SignError, SignOutput},
    },
    primitives::Blob,
    types::{MessageType, SigningAlgorithmSpec},
    Client,
};
use k256::ecdsa::{self, RecoveryId, VerifyingKey};
use std::fmt;

/// Amazon Web Services Key Management Service (AWS KMS) Ethereum signer.
///
/// The AWS Signer passes signing requests to the cloud service. AWS KMS keys are identified by a
/// UUID, the `key_id`.
///
/// Because the public key is unknown, we retrieve it on instantiation of the signer. This means
/// that the new function is `async` and must be called within some runtime.
///
/// Note that this signer only supports asynchronous operations. Calling a non-asynchronous method
/// will always return an error.
///
/// # Examples
///
/// ```no_run
/// use alloy_signer::Signer;
/// use alloy_signer_aws::AwsSigner;
/// use aws_config::BehaviorVersion;
///
/// # async fn test() {
/// let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
/// let client = aws_sdk_kms::Client::new(&config);
///
/// let key_id = "...".to_string();
/// let signer = AwsSigner::new(client, key_id).await.unwrap();
///
/// let message = vec![0, 1, 2, 3];
///
/// let sig = signer.sign_message_async(&message).await.unwrap();
/// assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
/// # }
/// ```
#[derive(Clone)]
pub struct AwsSigner {
    kms: Client,
    key_id: String,
    pubkey: VerifyingKey,
    address: Address,
}

impl fmt::Debug for AwsSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwsSigner")
            .field("key_id", &self.key_id)
            .field("pubkey", &hex::encode(self.pubkey.to_sec1_bytes()))
            .field("address", &self.address)
            .finish()
    }
}

/// Errors thrown by [`AwsSigner`].
#[derive(thiserror::Error, Debug)]
pub enum AwsSignerError {
    /// Thrown when the AWS KMS API returns a signing error.
    #[error(transparent)]
    Sign(#[from] SdkError<SignError>),
    /// Thrown when the AWS KMS API returns an error.
    #[error(transparent)]
    GetPublicKey(#[from] SdkError<GetPublicKeyError>),
    /// [`ecdsa`] error.
    #[error(transparent)]
    K256(#[from] ecdsa::Error),
    /// [`spki`] error.
    #[error(transparent)]
    Spki(#[from] spki::Error),
    /// [`hex`](mod@hex) error.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Thrown when the AWS KMS API returns a response without a signature.
    #[error("signature not found in response")]
    SignatureNotFound,
    /// Thrown when the AWS KMS API returns a response without a public key.
    #[error("public key not found in response")]
    PublicKeyNotFound,
}

#[async_trait::async_trait]
impl Signer for AwsSigner {
    #[inline]
    async fn sign_hash_async(&self, hash: &B256) -> Result<Signature> {
        self.sign_hash(hash).await.map_err(alloy_signer::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }
}

impl AwsSigner {
    /// Instantiate a new signer from an existing `Client` and key ID.
    ///
    /// Retrieves the public key from AWS and calculates the Ethereum address.
    #[instrument(skip(kms), ret)]
    pub async fn new(kms: Client, key_id: String) -> Result<AwsSigner, AwsSignerError> {
        let resp = request_get_pubkey(&kms, key_id.clone()).await?;
        let pubkey = decode_pubkey(resp)?;
        let address = alloy_signer::utils::public_key_to_address(&pubkey);
        debug!(?pubkey, %address, "instantiated AWS signer");
        Ok(Self { kms, key_id, pubkey, address })
    }

    /// Fetch the pubkey associated with a key ID.
    pub async fn get_pubkey_for_key(&self, key_id: String) -> Result<VerifyingKey, AwsSignerError> {
        request_get_pubkey(&self.kms, key_id).await.and_then(decode_pubkey)
    }

    /// Fetch the pubkey associated with this signer's key ID.
    #[inline]
    pub async fn get_pubkey(&self) -> Result<VerifyingKey, AwsSignerError> {
        self.get_pubkey_for_key(self.key_id.clone()).await
    }

    /// Signs a hash with the key associated with a key ID.
    #[instrument(skip(self), ret)]
    pub async fn sign_hash_with_key(
        &self,
        key_id: String,
        hash: &B256,
    ) -> Result<Signature, AwsSignerError> {
        let output = request_sign_hash(&self.kms, key_id, hash).await?;
        let sig = decode_signature(output)?;
        Ok(sig_from_recovery(sig, hash, &self.pubkey))
    }

    /// Signs a hash with this signer's key.
    #[inline]
    pub async fn sign_hash(&self, hash: &B256) -> Result<Signature, AwsSignerError> {
        self.sign_hash_with_key(self.key_id.clone(), hash).await
    }

    #[doc(hidden)]
    #[deprecated(note = "use `sign_hash_with_key` instead")]
    #[inline]
    pub async fn sign_digest_with_key(
        &self,
        key_id: String,
        digest: &B256,
    ) -> Result<Signature, AwsSignerError> {
        self.sign_hash_with_key(key_id, digest).await
    }

    #[doc(hidden)]
    #[deprecated(note = "use `sign_hash` instead")]
    #[inline]
    pub async fn sign_digest(&self, digest: &B256) -> Result<Signature, AwsSignerError> {
        self.sign_hash(digest).await
    }
}

#[instrument(skip(kms), ret)]
async fn request_get_pubkey(
    kms: &Client,
    key_id: String,
) -> Result<GetPublicKeyOutput, AwsSignerError> {
    kms.get_public_key().key_id(key_id).send().await.map_err(Into::into)
}

#[instrument(skip(kms), ret)]
async fn request_sign_hash(
    kms: &Client,
    key_id: String,
    hash: &B256,
) -> Result<SignOutput, AwsSignerError> {
    kms.sign()
        .key_id(key_id)
        .message(Blob::new(hash.as_slice()))
        .message_type(MessageType::Digest)
        .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
        .send()
        .await
        .map_err(Into::into)
}

/// Decode an AWS KMS Pubkey response.
fn decode_pubkey(resp: GetPublicKeyOutput) -> Result<VerifyingKey, AwsSignerError> {
    let raw = resp.public_key.as_ref().ok_or(AwsSignerError::PublicKeyNotFound)?;
    let spki = spki::SubjectPublicKeyInfoRef::try_from(raw.as_ref())?;
    let key = VerifyingKey::from_sec1_bytes(spki.subject_public_key.raw_bytes())?;
    Ok(key)
}

/// Decode an AWS KMS Signature response.
fn decode_signature(resp: SignOutput) -> Result<ecdsa::Signature, AwsSignerError> {
    let raw = resp.signature.as_ref().ok_or(AwsSignerError::SignatureNotFound)?;
    let sig = ecdsa::Signature::from_der(raw.as_ref())?;
    Ok(sig)
}

/// Gets the recovery ID by trial and error and creates a new [Signature].
fn sig_from_recovery(sig: ecdsa::Signature, hash: &B256, pubkey: &VerifyingKey) -> Signature {
    /// Makes a trial recovery to check whether an RSig corresponds to a known `VerifyingKey`.
    fn check_candidate(signature: &Signature, hash: &B256, pubkey: &VerifyingKey) -> bool {
        signature.recover_from_prehash(hash).map_or(false, |key| key == *pubkey)
    }

    let mut signature = Signature::new(sig, RecoveryId::from_byte(0).unwrap());
    if check_candidate(&signature, hash, pubkey) {
        return signature;
    }

    signature.set_v(1);
    if check_candidate(&signature, hash, pubkey) {
        return signature;
    }

    panic!("bad sig");
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::BehaviorVersion;

    #[tokio::test]
    async fn sign_message() {
        let Ok(key_id) = std::env::var("AWS_KEY_ID") else { return };
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = aws_sdk_kms::Client::new(&config);

        let signer = AwsSigner::new(client, key_id).await.unwrap();

        let message = b"hello";
        let sig = signer.sign_message_async(message).await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
    }
}
