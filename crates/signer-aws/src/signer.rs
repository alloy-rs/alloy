use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, Address, ChainId, Signature, B256};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
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
use k256::ecdsa::{self, VerifyingKey};
use std::fmt;

/// Amazon Web Services Key Management Service (AWS KMS) Ethereum signer.
///
/// Signing requests are sent to AWS KMS. The key must be an asymmetric `ECC_SECG_P256K1` key with
/// `SIGN_VERIFY` usage. `key_id` may be a key ID, key ARN, alias name, or alias ARN.
///
/// [`Self::new`] retrieves and caches the public key and derived Ethereum address. Signing performs
/// network I/O and must be awaited; this type does not implement [`alloy_signer::SignerSync`].
///
/// Constructing a signer requires `kms:GetPublicKey`; signing requires `kms:Sign`.
///
/// # Examples
///
/// ```no_run
/// use alloy_signer::Signer;
/// use alloy_signer_aws::{aws_config, aws_sdk_kms, AwsSigner};
///
/// # async fn test() {
/// let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
/// let client = aws_sdk_kms::Client::new(&config);
///
/// let key_id = std::env::var("AWS_KMS_KEY_ID").expect("AWS_KMS_KEY_ID");
/// let signer = AwsSigner::new(client, key_id, None).await.unwrap();
///
/// let message = b"hello from Alloy";
/// let sig = signer.sign_message(message).await.unwrap();
/// assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
/// # }
/// ```
#[derive(Clone)]
pub struct AwsSigner {
    kms: Client,
    key_id: String,
    pubkey: VerifyingKey,
    address: Address,
    chain_id: Option<ChainId>,
}

impl fmt::Debug for AwsSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwsSigner")
            .field("key_id", &self.key_id)
            .field("chain_id", &self.chain_id)
            .field("pubkey", &hex::encode(self.pubkey.to_sec1_bytes()))
            .field("address", &self.address)
            .finish()
    }
}

/// Errors thrown by [`AwsSigner`].
#[derive(Debug, thiserror::Error)]
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

    /// Failed to recover signature parity for the given digest and public key.
    #[error("failed to recover signature parity from KMS signature")]
    SignatureRecoveryFailed,
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl alloy_network::TxSigner<Signature> for AwsSigner {
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    #[doc(alias = "sign_tx")]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_hash(&tx.signature_hash()).await)
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl Signer for AwsSigner {
    #[allow(clippy::blocks_in_conditions)] // tracing::instrument on async fn
    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        self.sign_digest_inner(hash).await.map_err(alloy_signer::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

alloy_network::impl_into_wallet!(AwsSigner);

impl AwsSigner {
    /// Instantiate a new signer from an existing `Client` and key ID.
    ///
    /// `key_id` may be a key ID, key ARN, alias name, or alias ARN. This makes a
    /// `GetPublicKey` request, then caches the verifying key and derived Ethereum address.
    /// Prefer a stable key ID or ARN: retargeting an alias does not update the cached key or
    /// address, so signatures from the new target will fail parity recovery.
    ///
    /// `chain_id` affects transaction signing only. `Some(id)` fills an unset transaction chain ID
    /// and rejects a conflicting one before signing. It does not affect hash, message, or
    /// typed-data signing; `None` disables this signer-side check.
    #[instrument(skip(kms), err)]
    pub async fn new(
        kms: Client,
        key_id: String,
        chain_id: Option<ChainId>,
    ) -> Result<Self, AwsSignerError> {
        let resp = request_get_pubkey(&kms, key_id.clone()).await?;
        let pubkey = decode_pubkey(resp)?;
        let address = alloy_signer::utils::public_key_to_address(&pubkey);
        debug!(?pubkey, %address, "instantiated AWS signer");
        Ok(Self { kms, chain_id, key_id, pubkey, address })
    }

    /// Fetch the public key associated with a key ID.
    ///
    /// This makes a `GetPublicKey` request but does not change this signer's cached key or address.
    pub async fn get_pubkey_for_key(&self, key_id: String) -> Result<VerifyingKey, AwsSignerError> {
        request_get_pubkey(&self.kms, key_id).await.and_then(decode_pubkey)
    }

    /// Fetch the public key currently associated with this signer's key ID.
    ///
    /// This makes a `GetPublicKey` request but does not replace the key and address cached by
    /// [`Self::new`].
    pub async fn get_pubkey(&self) -> Result<VerifyingKey, AwsSignerError> {
        self.get_pubkey_for_key(self.key_id.clone()).await
    }

    /// Sign a precomputed 32-byte digest with the key associated with a key ID.
    ///
    /// The digest is sent to KMS as [`MessageType::Digest`] and is not hashed or prefixed again.
    /// The returned `(r, s)` signature is low-s normalized and has no recovery parity. Use
    /// [`Signer::sign_hash`] when an Alloy [`Signature`] with y-parity is required.
    pub async fn sign_digest_with_key(
        &self,
        key_id: String,
        digest: &B256,
    ) -> Result<ecdsa::Signature, AwsSignerError> {
        request_sign_digest(&self.kms, key_id, digest).await.and_then(decode_signature)
    }

    /// Sign a precomputed 32-byte digest with this signer's key.
    ///
    /// See [`Self::sign_digest_with_key`] for prehash and return-value semantics.
    pub async fn sign_digest(&self, digest: &B256) -> Result<ecdsa::Signature, AwsSignerError> {
        self.sign_digest_with_key(self.key_id.clone(), digest).await
    }

    /// Sign a digest with this signer's key and recover the correct y-parity.
    ///
    /// This does not apply EIP-155 itself. Transaction signing supplies a signature hash that
    /// already reflects the transaction's chain ID.
    #[instrument(err, skip(digest), fields(digest = %hex::encode(digest)))]
    async fn sign_digest_inner(&self, digest: &B256) -> Result<Signature, AwsSignerError> {
        let sig = self.sign_digest(digest).await?;
        sig_from_digest_bytes_trial_recovery(sig, digest, &self.pubkey)
    }
}

#[instrument(skip(kms), err)]
async fn request_get_pubkey(
    kms: &Client,
    key_id: String,
) -> Result<GetPublicKeyOutput, AwsSignerError> {
    kms.get_public_key().key_id(key_id).send().await.map_err(Into::into)
}

#[instrument(skip(kms, digest), fields(digest = %hex::encode(digest)), err)]
async fn request_sign_digest(
    kms: &Client,
    key_id: String,
    digest: &B256,
) -> Result<SignOutput, AwsSignerError> {
    kms.sign()
        .key_id(key_id)
        .message(Blob::new(digest.as_slice()))
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
    Ok(sig.normalize_s().unwrap_or(sig))
}

/// Recover an rsig from a signature under a known key by trial/error.
fn sig_from_digest_bytes_trial_recovery(
    sig: ecdsa::Signature,
    hash: &B256,
    pubkey: &VerifyingKey,
) -> Result<Signature, AwsSignerError> {
    let signature = Signature::from_signature_and_parity(sig, false);
    if check_candidate(&signature, hash, pubkey) {
        return Ok(signature);
    }

    let signature = signature.with_parity(true);
    if check_candidate(&signature, hash, pubkey) {
        return Ok(signature);
    }

    Err(AwsSignerError::SignatureRecoveryFailed)
}

/// Makes a trial recovery to check whether an RSig corresponds to a known `VerifyingKey`.
fn check_candidate(signature: &Signature, hash: &B256, pubkey: &VerifyingKey) -> bool {
    signature.recover_from_prehash(hash).map(|key| key == *pubkey).unwrap_or(false)
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

        let signer = AwsSigner::new(client, key_id, Some(1)).await.unwrap();

        let message = vec![0, 1, 2, 3];

        let sig = signer.sign_message(&message).await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
    }
}
