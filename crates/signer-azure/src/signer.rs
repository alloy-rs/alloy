use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, Address, ChainId, Signature, B256};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
use azure_core::credentials::TokenCredential;
use azure_security_keyvault_keys::{
    models::{
        CurveName, Key, KeyClientGetKeyOptions, KeyClientSignOptions, KeyOperationResult, KeyType,
        SignParameters, SignatureAlgorithm,
    },
    KeyClient, KeyClientOptions, ResourceExt, ResourceId,
};
use k256::ecdsa::{self, RecoveryId, VerifyingKey};
use std::{fmt, sync::Arc};

/// Length in bytes of a secp256k1 affine coordinate.
const COORDINATE_LEN: usize = 32;

/// Azure Key Vault Ethereum signer.
///
/// Signing requests are sent to Azure Key Vault or Azure Managed HSM. The key must be an elliptic
/// curve key (`EC` or `EC-HSM`) on the `P-256K` (secp256k1) curve.
///
/// [`Self::new`] retrieves and caches the public key and derived Ethereum address, and pins the key
/// version: every signature is produced by that version, even if the key is later rotated. Signing
/// performs network I/O and must be awaited; this type does not implement
/// [`alloy_signer::SignerSync`].
///
/// Key Vault returns `ES256K` signatures as raw `r || s` without a recovery ID. Signatures are
/// normalized to low-s form and their y-parity is recovered against the cached public key.
///
/// Constructing a signer requires the `keys/get` permission; signing requires `keys/sign`.
///
/// # Examples
///
/// ```no_run
/// use alloy_signer::Signer;
/// use alloy_signer_azure::{azure_identity::DeveloperToolsCredential, AzureSigner};
///
/// # async fn test() {
/// let credential = DeveloperToolsCredential::new(None).unwrap();
///
/// // For example `https://my-vault.vault.azure.net/keys/my-key/<version>`.
/// let key_id = std::env::var("AZURE_KEY_VAULT_KEY_ID").expect("AZURE_KEY_VAULT_KEY_ID");
/// let signer = AzureSigner::from_key_id(&key_id, credential, None, None).await.unwrap();
///
/// let message = b"hello from Alloy";
/// let sig = signer.sign_message(message).await.unwrap();
/// assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
/// # }
/// ```
#[derive(Clone)]
pub struct AzureSigner {
    client: Arc<KeyClient>,
    key_name: String,
    key_version: String,
    pubkey: VerifyingKey,
    address: Address,
    chain_id: Option<ChainId>,
}

impl fmt::Debug for AzureSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureSigner")
            .field("key_name", &self.key_name)
            .field("key_version", &self.key_version)
            .field("chain_id", &self.chain_id)
            .field("pubkey", &hex::encode(self.pubkey.to_sec1_bytes()))
            .field("address", &self.address)
            .finish()
    }
}

/// Errors thrown by [`AzureSigner`].
#[derive(Debug, thiserror::Error)]
pub enum AzureSignerError {
    /// Thrown when the Azure SDK returns an error.
    #[error(transparent)]
    Azure(#[from] azure_core::Error),
    /// [`ecdsa`] error.
    #[error(transparent)]
    K256(#[from] ecdsa::Error),
    /// Thrown when a key identifier is not a Key Vault key URL.
    #[error(
        "invalid key identifier `{key_id}`, expected \
         `https://<vault>.vault.azure.net/keys/<name>[/<version>]`: {source}"
    )]
    InvalidKeyId {
        /// The invalid key identifier.
        key_id: String,
        /// The parse error.
        source: azure_core::Error,
    },
    /// Thrown when the key is not an elliptic curve key on the `P-256K` curve.
    #[error("unsupported key type {kty:?} or curve {crv:?}, expected EC or EC-HSM on P-256K")]
    UnsupportedKey {
        /// The key type returned by Key Vault.
        kty: Option<KeyType>,
        /// The curve returned by Key Vault.
        crv: Option<CurveName>,
    },
    /// Thrown when Key Vault returns a response without a public key.
    #[error("public key not found in response")]
    PublicKeyNotFound,
    /// Thrown when Key Vault returns a public key coordinate longer than 32 bytes.
    #[error("invalid public key coordinate length: {0} bytes")]
    InvalidCoordinateLength(usize),
    /// Thrown when Key Vault returns a response without a key version.
    #[error("key version not found in response")]
    KeyVersionNotFound,
    /// Thrown when Key Vault returns a key version other than the expected one.
    #[error("key version mismatch: expected {expected}, got {actual}")]
    KeyVersionMismatch {
        /// The expected key version.
        expected: String,
        /// The key version returned by Key Vault.
        actual: String,
    },
    /// Thrown when Key Vault returns a response without a signature.
    #[error("signature not found in response")]
    SignatureNotFound,

    /// Failed to recover signature parity for the given digest and public key.
    #[error("failed to recover signature parity from Key Vault signature")]
    SignatureRecoveryFailed,
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl alloy_network::TxSigner<Signature> for AzureSigner {
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
impl Signer for AzureSigner {
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

alloy_network::impl_into_wallet!(AzureSigner);

impl AzureSigner {
    /// Instantiate a new signer from an existing [`KeyClient`] and key name.
    ///
    /// This makes a get-key request, then caches the verifying key and derived Ethereum address.
    /// If `key_version` is `None`, the current version of the key is resolved and pinned, so a
    /// later key rotation does not change which key signs.
    ///
    /// `chain_id` affects transaction signing only. `Some(id)` fills an unset transaction chain ID
    /// and rejects a conflicting one before signing. It does not affect hash, message, or
    /// typed-data signing; `None` disables this signer-side check.
    #[instrument(skip(client), err)]
    pub async fn new(
        client: KeyClient,
        key_name: String,
        key_version: Option<String>,
        chain_id: Option<ChainId>,
    ) -> Result<Self, AzureSignerError> {
        let key = request_get_pubkey(&client, &key_name, key_version.clone()).await?;
        let key_version = resolve_key_version(key_version.as_deref(), &key)?;
        let pubkey = decode_pubkey(&key)?;
        let address = alloy_signer::utils::public_key_to_address(&pubkey);
        debug!(?pubkey, %address, %key_version, "instantiated Azure signer");
        Ok(Self { client: Arc::new(client), key_name, key_version, pubkey, address, chain_id })
    }

    /// Instantiate a new signer from a Key Vault key identifier and a credential.
    ///
    /// `key_id` is a key identifier such as `https://my-vault.vault.azure.net/keys/my-key` or
    /// `https://my-hsm.managedhsm.azure.net/keys/my-key/<version>`. The version segment is
    /// optional; see [`Self::new`] for how the key version is pinned.
    pub async fn from_key_id(
        key_id: &str,
        credential: Arc<dyn TokenCredential>,
        options: Option<KeyClientOptions>,
        chain_id: Option<ChainId>,
    ) -> Result<Self, AzureSignerError> {
        let id = key_id.parse::<ResourceId>().map_err(|source| AzureSignerError::InvalidKeyId {
            key_id: key_id.to_string(),
            source,
        })?;
        let client = KeyClient::new(&id.vault_url, credential, options)?;
        Self::new(client, id.name, id.version, chain_id).await
    }

    /// Returns the name of the key used by this signer.
    #[inline]
    pub fn key_name(&self) -> &str {
        &self.key_name
    }

    /// Returns the pinned key version used by this signer.
    #[inline]
    pub fn key_version(&self) -> &str {
        &self.key_version
    }

    /// Fetch the public key of this signer's pinned key version.
    ///
    /// This makes a get-key request but does not change this signer's cached key or address.
    pub async fn get_pubkey(&self) -> Result<VerifyingKey, AzureSignerError> {
        request_get_pubkey(&self.client, &self.key_name, Some(self.key_version.clone()))
            .await
            .and_then(|key| decode_pubkey(&key))
    }

    /// Sign a precomputed 32-byte digest with this signer's pinned key version.
    ///
    /// Key Vault signs the digest as given and does not hash or prefix it again. The returned
    /// `(r, s)` signature is low-s normalized and has no recovery parity. Use
    /// [`Signer::sign_hash`] when an Alloy [`Signature`] with y-parity is required.
    pub async fn sign_digest(&self, digest: &B256) -> Result<ecdsa::Signature, AzureSignerError> {
        request_sign_digest(&self.client, &self.key_name, &self.key_version, digest)
            .await
            .and_then(decode_signature)
    }

    /// Sign a digest with this signer's key and recover the correct y-parity.
    ///
    /// This does not apply EIP-155 itself. Transaction signing supplies a signature hash that
    /// already reflects the transaction's chain ID.
    #[instrument(err, skip(digest), fields(digest = %hex::encode(digest)))]
    async fn sign_digest_inner(&self, digest: &B256) -> Result<Signature, AzureSignerError> {
        let sig = self.sign_digest(digest).await?;
        // Ethereum signatures cannot encode x-reduced recovery IDs.
        let recid = RecoveryId::trial_recovery_from_prehash(&self.pubkey, digest.as_slice(), &sig)
            .ok()
            .filter(|recid| !recid.is_x_reduced())
            .ok_or(AzureSignerError::SignatureRecoveryFailed)?;
        Ok(Signature::from_signature_and_parity(sig, recid.is_y_odd()))
    }
}

#[instrument(skip(client), err)]
async fn request_get_pubkey(
    client: &KeyClient,
    key_name: &str,
    key_version: Option<String>,
) -> Result<Key, AzureSignerError> {
    let options = KeyClientGetKeyOptions { key_version, ..Default::default() };
    client.get_key(key_name, Some(options)).await?.into_model().map_err(Into::into)
}

#[instrument(skip(client, digest), fields(digest = %hex::encode(digest)), err)]
async fn request_sign_digest(
    client: &KeyClient,
    key_name: &str,
    key_version: &str,
    digest: &B256,
) -> Result<KeyOperationResult, AzureSignerError> {
    let parameters = SignParameters {
        algorithm: Some(SignatureAlgorithm::Es256K),
        value: Some(digest.to_vec()),
    };
    let options =
        KeyClientSignOptions { key_version: Some(key_version.to_string()), ..Default::default() };
    let resp = client.sign(key_name, parameters.try_into()?, Some(options)).await?.into_model()?;
    resolve_key_version(Some(key_version), &resp)?;
    Ok(resp)
}

/// Resolve the key version of a Key Vault response, checking it against `expected` if given.
///
/// Key versions are compared case-insensitively, as Key Vault does.
fn resolve_key_version(
    expected: Option<&str>,
    resp: &impl ResourceExt,
) -> Result<String, AzureSignerError> {
    let actual = resp.resource_id()?.version.ok_or(AzureSignerError::KeyVersionNotFound)?;
    match expected {
        Some(expected) if !expected.eq_ignore_ascii_case(&actual) => {
            Err(AzureSignerError::KeyVersionMismatch { expected: expected.to_string(), actual })
        }
        _ => Ok(actual),
    }
}

/// Decode a Key Vault key into a secp256k1 verifying key.
fn decode_pubkey(key: &Key) -> Result<VerifyingKey, AzureSignerError> {
    let jwk = key.key.as_ref().ok_or(AzureSignerError::PublicKeyNotFound)?;
    let supported_kty = matches!(jwk.kty, Some(KeyType::Ec | KeyType::EcHsm));
    if !supported_kty || !matches!(jwk.crv, Some(CurveName::P256K)) {
        return Err(AzureSignerError::UnsupportedKey {
            kty: jwk.kty.clone(),
            crv: jwk.crv.clone(),
        });
    }
    let (Some(x), Some(y)) = (jwk.x.as_deref(), jwk.y.as_deref()) else {
        return Err(AzureSignerError::PublicKeyNotFound);
    };
    let mut sec1 = [0u8; 1 + 2 * COORDINATE_LEN];
    sec1[0] = 0x04;
    write_coordinate(&mut sec1[1..1 + COORDINATE_LEN], x)?;
    write_coordinate(&mut sec1[1 + COORDINATE_LEN..], y)?;
    Ok(VerifyingKey::from_sec1_bytes(&sec1)?)
}

/// Left-pad a big-endian JWK coordinate, which may omit leading zero bytes, into `out`.
fn write_coordinate(out: &mut [u8], coordinate: &[u8]) -> Result<(), AzureSignerError> {
    let Some(offset) = out.len().checked_sub(coordinate.len()) else {
        return Err(AzureSignerError::InvalidCoordinateLength(coordinate.len()));
    };
    out[offset..].copy_from_slice(coordinate);
    Ok(())
}

/// Decode a Key Vault signature response and normalize it to low-s form.
///
/// Key Vault returns `ES256K` signatures as raw 64-byte `r || s`, not DER.
fn decode_signature(resp: KeyOperationResult) -> Result<ecdsa::Signature, AzureSignerError> {
    let raw = resp.result.as_ref().ok_or(AzureSignerError::SignatureNotFound)?;
    let sig = ecdsa::Signature::from_slice(raw)?;
    Ok(sig.normalize_s().unwrap_or(sig))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::TxLegacy;
    use alloy_primitives::{b256, U256};
    use azure_core::{
        base64,
        credentials::{AccessToken, TokenRequestOptions},
        http::{
            headers::{Headers, AUTHORIZATION, WWW_AUTHENTICATE},
            AsyncRawResponse, Body, ClientOptions, HttpClient, Request, StatusCode, Transport,
        },
        time::{Duration, OffsetDateTime},
        Bytes,
    };
    use azure_identity::DeveloperToolsCredential;
    use azure_security_keyvault_keys::models::JsonWebKey;
    use k256::ecdsa::SigningKey;
    use serde_json::json;
    use std::sync::Mutex;

    const VAULT_URL: &str = "https://mock.vault.azure.net";
    const KEY_NAME: &str = "eth-key";
    const KEY_VERSION: &str = "0123456789abcdef";
    const SCOPE: &str = "https://vault.azure.net/.default";
    const CHALLENGE: &str = concat!(
        r#"Bearer authorization="https://login.microsoftonline.com/tenant", "#,
        r#"resource="https://vault.azure.net""#
    );

    /// Returns the key identifier of the mock key, with an optional version.
    fn kid(version: &str) -> String {
        format!("{VAULT_URL}/keys/{KEY_NAME}/{version}")
    }

    /// Returns a deterministic signing key derived from `seed`.
    fn signing_key(seed: u64) -> SigningKey {
        SigningKey::from_slice(&U256::from(seed).to_be_bytes::<32>()).unwrap()
    }

    /// Returns the JSON Web Key representation of `key` as returned by Key Vault.
    fn jwk(key: &VerifyingKey, version: &str) -> JsonWebKey {
        let point = key.to_encoded_point(false);
        JsonWebKey {
            kid: Some(kid(version)),
            kty: Some(KeyType::Ec),
            crv: Some(CurveName::P256K),
            x: Some(point.x().unwrap().to_vec()),
            y: Some(point.y().unwrap().to_vec()),
            ..Default::default()
        }
    }

    /// Wraps `jwk` in a get-key response.
    fn key(jwk: JsonWebKey) -> Key {
        let mut key = Key::default();
        key.key = Some(jwk);
        key
    }

    /// Returns a sign response for `sig` from the key version `version`.
    fn sign_result(sig: &[u8], version: &str) -> KeyOperationResult {
        let mut resp = KeyOperationResult::default();
        resp.kid = Some(kid(version));
        resp.result = Some(sig.to_vec());
        resp
    }

    /// Returns the high-s form of a low-s signature.
    fn to_high_s(sig: ecdsa::Signature) -> ecdsa::Signature {
        let (r, s) = sig.split_scalars();
        ecdsa::Signature::from_scalars(r.to_bytes(), (-*s).to_bytes()).unwrap()
    }

    #[derive(Debug)]
    struct MockCredential;

    #[async_trait]
    impl TokenCredential for MockCredential {
        async fn get_token(
            &self,
            scopes: &[&str],
            _options: Option<TokenRequestOptions<'_>>,
        ) -> azure_core::Result<AccessToken> {
            assert_eq!(scopes, [SCOPE]);
            Ok(AccessToken::new("token", OffsetDateTime::now_utc() + Duration::hours(1)))
        }
    }

    /// In-memory Key Vault that implements the get-key and sign operations for versioned keys.
    #[derive(Debug)]
    struct MockVault {
        /// Key versions, the last of which is the current version.
        versions: Mutex<Vec<(String, SigningKey)>>,
        /// Returns high-s signatures when set.
        high_s: bool,
        /// Overrides the key version reported in sign responses.
        sign_version: Option<&'static str>,
        requests: Mutex<Vec<String>>,
    }

    impl MockVault {
        fn new(key: SigningKey) -> Self {
            Self {
                versions: Mutex::new(vec![(KEY_VERSION.to_string(), key)]),
                high_s: false,
                sign_version: None,
                requests: Mutex::default(),
            }
        }

        fn rotate(&self, version: &str, key: SigningKey) {
            self.versions.lock().unwrap().push((version.to_string(), key));
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        fn respond(&self, request: &Request) -> serde_json::Value {
            let path = request.url().path().trim_start_matches(&format!("/keys/{KEY_NAME}/"));
            let (version, operation) = path.split_once('/').unwrap_or((path, ""));
            let versions = self.versions.lock().unwrap();
            let (version, key) = if version.is_empty() {
                versions.last().unwrap()
            } else {
                versions.iter().find(|(v, _)| v == version).unwrap()
            };

            if operation != "sign" {
                return json!({ "key": jwk(key.verifying_key(), version) });
            }
            let Body::Bytes(body) = request.body() else { panic!("unexpected streaming body") };
            let parameters = serde_json::from_slice::<SignParameters>(body).unwrap();
            assert_eq!(parameters.algorithm, Some(SignatureAlgorithm::Es256K));
            let (sig, _) = key.sign_prehash_recoverable(&parameters.value.unwrap()).unwrap();
            let sig = if self.high_s { to_high_s(sig) } else { sig };
            let version = self.sign_version.unwrap_or(version);
            json!({ "kid": kid(version), "value": base64::encode_url_safe(sig.to_bytes()) })
        }
    }

    #[async_trait]
    impl HttpClient for MockVault {
        async fn execute_request(&self, request: &Request) -> azure_core::Result<AsyncRawResponse> {
            // Key Vault answers the first, unauthenticated request with a bearer challenge.
            if request.headers().get_optional_str(&AUTHORIZATION).is_none() {
                let mut headers = Headers::new();
                headers.insert(WWW_AUTHENTICATE, CHALLENGE);
                return Ok(AsyncRawResponse::from_bytes(
                    StatusCode::Unauthorized,
                    headers,
                    Bytes::new(),
                ));
            }
            let line = format!("{} {}", request.method(), request.url().path());
            self.requests.lock().unwrap().push(line);
            let body = serde_json::to_vec(&self.respond(request)).unwrap();
            Ok(AsyncRawResponse::from_bytes(StatusCode::Ok, Headers::new(), body))
        }
    }

    async fn mock_signer(vault: Arc<MockVault>, key_id: &str) -> AzureSigner {
        let options = KeyClientOptions {
            client_options: ClientOptions {
                transport: Some(Transport::new(vault)),
                ..Default::default()
            },
            ..Default::default()
        };
        AzureSigner::from_key_id(key_id, Arc::new(MockCredential), Some(options), Some(1))
            .await
            .unwrap()
    }

    #[test]
    fn decode_pubkey_accepts_ec_and_ec_hsm_keys() {
        let signing_key = signing_key(1);
        let mut jwk = jwk(signing_key.verifying_key(), KEY_VERSION);
        assert_eq!(decode_pubkey(&key(jwk.clone())).unwrap(), *signing_key.verifying_key());

        jwk.kty = Some(KeyType::EcHsm);
        assert_eq!(decode_pubkey(&key(jwk)).unwrap(), *signing_key.verifying_key());
    }

    #[test]
    fn decode_pubkey_pads_short_coordinates() {
        // JWK coordinates may omit leading zero bytes; find a key whose x coordinate has one.
        let signing_key = (1..)
            .map(signing_key)
            .find(|key| key.verifying_key().to_encoded_point(false).x().unwrap()[0] == 0)
            .unwrap();
        let mut jwk = jwk(signing_key.verifying_key(), KEY_VERSION);
        jwk.x = Some(jwk.x.unwrap()[1..].to_vec());
        assert_eq!(decode_pubkey(&key(jwk)).unwrap(), *signing_key.verifying_key());
    }

    #[test]
    fn decode_pubkey_rejects_unsupported_keys() {
        let jwk = jwk(signing_key(1).verifying_key(), KEY_VERSION);

        let p256 = JsonWebKey { crv: Some(CurveName::P256), ..jwk.clone() };
        assert!(matches!(decode_pubkey(&key(p256)), Err(AzureSignerError::UnsupportedKey { .. })));

        let rsa = JsonWebKey { kty: Some(KeyType::Rsa), ..jwk.clone() };
        assert!(matches!(decode_pubkey(&key(rsa)), Err(AzureSignerError::UnsupportedKey { .. })));

        let missing = JsonWebKey { y: None, ..jwk.clone() };
        assert!(matches!(decode_pubkey(&key(missing)), Err(AzureSignerError::PublicKeyNotFound)));

        let long = JsonWebKey { x: Some(vec![1; COORDINATE_LEN + 1]), ..jwk };
        assert!(matches!(
            decode_pubkey(&key(long)),
            Err(AzureSignerError::InvalidCoordinateLength(33))
        ));

        assert!(matches!(decode_pubkey(&Key::default()), Err(AzureSignerError::PublicKeyNotFound)));
    }

    #[test]
    fn decode_signature_normalizes_high_s() {
        let digest = b256!("0x0101010101010101010101010101010101010101010101010101010101010101");
        let (sig, _) = signing_key(1).sign_prehash_recoverable(digest.as_slice()).unwrap();
        let high_s = to_high_s(sig);
        assert!(high_s.normalize_s().is_some());

        assert_eq!(decode_signature(sign_result(&high_s.to_bytes(), KEY_VERSION)).unwrap(), sig);
        assert_eq!(decode_signature(sign_result(&sig.to_bytes(), KEY_VERSION)).unwrap(), sig);
    }

    #[test]
    fn decode_signature_rejects_invalid_responses() {
        for len in [0, 63, 65] {
            let resp = sign_result(&vec![1; len], KEY_VERSION);
            assert!(matches!(decode_signature(resp), Err(AzureSignerError::K256(_))));
        }
        assert!(matches!(
            decode_signature(KeyOperationResult::default()),
            Err(AzureSignerError::SignatureNotFound)
        ));
    }

    #[test]
    fn resolve_key_version_checks_expected_version() {
        let resp = sign_result(&[], KEY_VERSION);
        assert_eq!(resolve_key_version(None, &resp).unwrap(), KEY_VERSION);
        assert_eq!(resolve_key_version(Some(KEY_VERSION), &resp).unwrap(), KEY_VERSION);
        assert_eq!(
            resolve_key_version(Some(&KEY_VERSION.to_uppercase()), &resp).unwrap(),
            KEY_VERSION
        );
        assert!(matches!(
            resolve_key_version(Some("other"), &resp),
            Err(AzureSignerError::KeyVersionMismatch { .. })
        ));
        assert!(matches!(
            resolve_key_version(None, &sign_result(&[], "")),
            Err(AzureSignerError::KeyVersionNotFound)
        ));
        assert!(matches!(
            resolve_key_version(None, &KeyOperationResult::default()),
            Err(AzureSignerError::Azure(_))
        ));
    }

    #[tokio::test]
    async fn from_key_id_rejects_non_key_identifiers() {
        for key_id in ["not a url", "https://mock.vault.azure.net/secrets/eth-key"] {
            let err = AzureSigner::from_key_id(key_id, Arc::new(MockCredential), None, None)
                .await
                .unwrap_err();
            assert!(matches!(err, AzureSignerError::InvalidKeyId { .. }));
        }
    }

    #[tokio::test]
    async fn mock_vault_pins_version_and_signs() {
        let signing_key = signing_key(42);
        let vault = Arc::new(MockVault::new(signing_key.clone()));
        let signer = mock_signer(vault.clone(), &kid("")).await;

        let address = alloy_signer::utils::public_key_to_address(signing_key.verifying_key());
        assert_eq!(signer.address(), address);
        assert_eq!(signer.key_name(), KEY_NAME);
        assert_eq!(signer.key_version(), KEY_VERSION);

        let message = b"hello from Alloy";
        let sig = signer.sign_message(message).await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());

        // The unversioned key identifier resolves the current version once, then signs with the
        // pinned version.
        assert_eq!(
            vault.requests(),
            [format!("GET /keys/{KEY_NAME}/"), format!("POST /keys/{KEY_NAME}/{KEY_VERSION}/sign")]
        );
    }

    #[tokio::test]
    async fn mock_vault_rotation_keeps_pinned_version() {
        let vault = Arc::new(MockVault::new(signing_key(5)));
        let signer = mock_signer(vault.clone(), &kid("")).await;
        vault.rotate("fedcba9876543210", signing_key(6));

        let digest = B256::repeat_byte(1);
        let sig = signer.sign_hash(&digest).await.unwrap();
        assert_eq!(sig.recover_address_from_prehash(&digest).unwrap(), signer.address());
        assert_eq!(signer.get_pubkey().await.unwrap(), *signing_key(5).verifying_key());

        // A new signer for the same unversioned key identifier pins the rotated version.
        let rotated = mock_signer(vault, &kid("")).await;
        assert_eq!(rotated.key_version(), "fedcba9876543210");
        assert_ne!(rotated.address(), signer.address());
    }

    #[tokio::test]
    async fn mock_vault_high_s_signatures_recover() {
        let vault = Arc::new(MockVault { high_s: true, ..MockVault::new(signing_key(7)) });
        let signer = mock_signer(vault, &kid(KEY_VERSION)).await;

        for i in 0..8u8 {
            let digest = B256::repeat_byte(i);
            let sig = signer.sign_hash(&digest).await.unwrap();
            assert!(sig.normalize_s().is_none());
            assert_eq!(sig.recover_address_from_prehash(&digest).unwrap(), signer.address());
        }
    }

    #[tokio::test]
    async fn mock_vault_signs_transactions() {
        let vault = Arc::new(MockVault::new(signing_key(9)));
        let signer = mock_signer(vault, &kid(KEY_VERSION)).await;

        let mut tx = TxLegacy { gas_limit: 21_000, ..Default::default() };
        let sig = alloy_network::TxSigner::sign_transaction(&signer, &mut tx).await.unwrap();
        assert_eq!(tx.chain_id, Some(1));
        let hash = tx.signature_hash();
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), signer.address());

        let mut tx = TxLegacy { chain_id: Some(2), ..Default::default() };
        assert!(alloy_network::TxSigner::sign_transaction(&signer, &mut tx).await.is_err());
    }

    #[tokio::test]
    async fn mock_vault_rejects_other_key_version() {
        let vault =
            Arc::new(MockVault { sign_version: Some("other"), ..MockVault::new(signing_key(3)) });
        let signer = mock_signer(vault, &kid(KEY_VERSION)).await;

        let err = signer.sign_digest(&B256::ZERO).await.unwrap_err();
        assert!(matches!(err, AzureSignerError::KeyVersionMismatch { .. }));
    }

    #[tokio::test]
    async fn sign_message() {
        let Ok(key_id) = std::env::var("AZURE_KEY_VAULT_KEY_ID") else { return };
        let credential = DeveloperToolsCredential::new(None).unwrap();
        let signer = AzureSigner::from_key_id(&key_id, credential, None, Some(1)).await.unwrap();

        let message = vec![0, 1, 2, 3];

        let sig = signer.sign_message(&message).await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
    }
}
