use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, Address, ChainId, Signature, B256};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signer};
use async_trait::async_trait;
use std::fmt;
use turnkey_client::{
    generated::{
        immutable::common::v1::{HashFunction, PayloadEncoding},
        SignRawPayloadIntentV2,
    },
    TurnkeyClient, TurnkeyClientError,
};
/// Turnkey signer implementation for Alloy.
///
/// The Turnkey Signer passes signing requests to the Turnkey secure key management infrastructure.
/// This implementation uses Turnkey's sign_raw_payload API with HASH_FUNCTION_NO_OP for simplicity.
///
/// The signer is initialized with a user-provided address that corresponds to a key in your Turnkey
/// organization. This follows the Turnkey team's recommendation for an MVP implementation.
///
/// Note that this signer only supports asynchronous operations. Calling a non-asynchronous method
/// will always return an error.
///
/// # Examples
///
/// ```no_run
/// use alloy_primitives::Address;
/// use alloy_signer::Signer;
/// use alloy_signer_turnkey::{
///     turnkey_client::{TurnkeyClient, TurnkeyP256ApiKey},
///     TurnkeySigner,
/// };
///
/// # async fn test() {
/// let api_key =
///     TurnkeyP256ApiKey::from_strings("private_key_hex", None).expect("api key creation failed");
/// let client = TurnkeyClient::builder().api_key(api_key).build().expect("client builder failed");
/// let org_id = "your-org-id".to_string();
/// let address = "0x1234567890123456789012345678901234567890".parse::<Address>().unwrap();
/// let chain_id = Some(1);
/// let signer = TurnkeySigner::new(client, org_id, address, chain_id);
///
/// let message = vec![0, 1, 2, 3];
/// let sig = signer.sign_message(&message).await.unwrap();
/// assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
/// # }
/// ```
pub struct TurnkeySigner {
    client: TurnkeyClient,
    organization_id: String,
    address: Address,
    chain_id: Option<ChainId>,
}

impl fmt::Debug for TurnkeySigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TurnkeySigner")
            .field("organization_id", &self.organization_id)
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

/// Errors that can occur when using the Turnkey signer.
#[derive(Debug, thiserror::Error)]
pub enum TurnkeySignerError {
    /// Turnkey client error.
    #[error(transparent)]
    TurnkeyClient(#[from] TurnkeyClientError),
    /// Invalid hex string in response.
    #[error("invalid hex string: {0}")]
    Hex(#[from] hex::FromHexError),
    /// Signature not found in response.
    #[error("signature not found in response")]
    SignatureNotFound,
    /// Invalid signature format received from Turnkey.
    #[error("invalid signature format")]
    InvalidSignature,
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl alloy_network::TxSigner<Signature> for TurnkeySigner {
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
impl Signer for TurnkeySigner {
    #[instrument(err)]
    #[allow(clippy::blocks_in_conditions)]
    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        let response = self
            .client
            .sign_raw_payload(
                self.organization_id.clone(),
                self.client.current_timestamp(),
                SignRawPayloadIntentV2 {
                    sign_with: self.address.to_string(),
                    payload: hex::encode(hash),
                    encoding: PayloadEncoding::Hexadecimal,
                    hash_function: HashFunction::NoOp,
                },
            )
            .await
            .map_err(|e| alloy_signer::Error::other(TurnkeySignerError::TurnkeyClient(e)))?;

        // Parse r, s, v from response
        let r_bytes = hex::decode(&response.r)
            .map_err(|e| alloy_signer::Error::other(TurnkeySignerError::Hex(e)))?;
        let s_bytes = hex::decode(&response.s)
            .map_err(|e| alloy_signer::Error::other(TurnkeySignerError::Hex(e)))?;
        let v_bytes = hex::decode(&response.v)
            .map_err(|e| alloy_signer::Error::other(TurnkeySignerError::Hex(e)))?;

        if r_bytes.len() != 32 || s_bytes.len() != 32 || v_bytes.is_empty() {
            return Err(alloy_signer::Error::other(TurnkeySignerError::InvalidSignature));
        }

        let v = v_bytes[0];
        let mut sig_bytes = [0u8; 65];
        sig_bytes[..32].copy_from_slice(&r_bytes);
        sig_bytes[32..64].copy_from_slice(&s_bytes);
        sig_bytes[64] = v;

        Signature::try_from(&sig_bytes[..]).map_err(alloy_signer::Error::other)
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

alloy_network::impl_into_wallet!(TurnkeySigner);

impl TurnkeySigner {
    /// Instantiate a new signer from an existing client, organization ID, and address.
    ///
    /// This follows Turnkey team's recommendation: use the address directly for signing
    /// via sign_raw_payload, avoiding the need for async key fetching.
    pub const fn new(
        client: TurnkeyClient,
        organization_id: String,
        address: Address,
        chain_id: Option<ChainId>,
    ) -> Self {
        Self { client, organization_id, address, chain_id }
    }

    /// Instantiate a new signer from API credentials, organization ID, and address.
    ///
    /// This is a convenience constructor that builds the Turnkey client from
    /// an API private key string.
    pub fn from_api_key(
        api_private_key: &str,
        organization_id: String,
        address: Address,
        chain_id: Option<ChainId>,
    ) -> Result<Self, TurnkeySignerError> {
        let api_key = turnkey_client::TurnkeyP256ApiKey::from_strings(api_private_key, None)
            .map_err(|err| TurnkeySignerError::TurnkeyClient(TurnkeyClientError::from(err)))?;
        let client = TurnkeyClient::builder().api_key(api_key).build()?;
        Ok(Self::new(client, organization_id, address, chain_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::VerifyingKey;
    use turnkey_client::TurnkeyP256ApiKey;

    #[tokio::test]
    async fn sign_message() {
        // Environment check - return early if credentials missing (AWS/GCP pattern)
        let Ok(org_id) = std::env::var("TURNKEY_ORGANIZATION_ID") else { return };
        let Ok(api_private_key) = std::env::var("TURNKEY_API_PRIVATE_KEY") else { return };
        let Ok(address_str) = std::env::var("TURNKEY_ADDRESS") else { return };

        // Create API key and client using official SDK
        let api_key = TurnkeyP256ApiKey::from_strings(&api_private_key, None)
            .expect("api key creation failed");

        let client =
            TurnkeyClient::builder().api_key(api_key).build().expect("client builder failed");

        let address = address_str.parse::<Address>().expect("invalid test address");
        let signer = TurnkeySigner::new(client, org_id, address, Some(1));

        // Standard test payload (matches AWS/GCP exactly)
        let message = vec![0, 1, 2, 3];

        // Execute signing and verify recovery (AWS/GCP pattern)
        let sig = signer.sign_message(&message).await.unwrap();
        assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
    }

    #[tokio::test]
    async fn sign_hash() {
        let Ok(org_id) = std::env::var("TURNKEY_ORGANIZATION_ID") else { return };
        let Ok(api_private_key) = std::env::var("TURNKEY_API_PRIVATE_KEY") else { return };
        let Ok(address_str) = std::env::var("TURNKEY_ADDRESS") else { return };

        let api_key = TurnkeyP256ApiKey::from_strings(&api_private_key, None)
            .expect("api key creation failed");

        let client =
            TurnkeyClient::builder().api_key(api_key).build().expect("client builder failed");

        let address = address_str.parse::<Address>().expect("invalid test address");
        let signer = TurnkeySigner::new(client, org_id, address, Some(1));

        // Test direct hash signing (core functionality)
        let hash = B256::from([1u8; 32]);
        let sig = signer.sign_hash(&hash).await.unwrap();

        // Verify signature recovery
        let recovered: VerifyingKey = sig.recover_from_prehash(&hash).unwrap();
        assert_eq!(alloy_signer::utils::public_key_to_address(&recovered), signer.address());
    }

    #[tokio::test]
    async fn signer_properties() {
        let Ok(org_id) = std::env::var("TURNKEY_ORGANIZATION_ID") else { return };
        let Ok(api_private_key) = std::env::var("TURNKEY_API_PRIVATE_KEY") else { return };
        let Ok(address_str) = std::env::var("TURNKEY_ADDRESS") else { return };

        let api_key = TurnkeyP256ApiKey::from_strings(&api_private_key, None)
            .expect("api key creation failed");

        let client =
            TurnkeyClient::builder().api_key(api_key).build().expect("client builder failed");

        let address = address_str.parse::<Address>().expect("invalid test address");
        let mut signer = TurnkeySigner::new(client, org_id, address, Some(1));

        // Test address property
        assert_eq!(signer.address(), address);

        // Test chain_id property
        assert_eq!(signer.chain_id(), Some(1));

        // Test chain_id mutation
        signer.set_chain_id(Some(42));
        assert_eq!(signer.chain_id(), Some(42));

        signer.set_chain_id(None);
        assert_eq!(signer.chain_id(), None);
    }
}
