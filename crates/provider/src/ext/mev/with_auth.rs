use crate::{ext::FLASHBOTS_SIGNATURE_HEADER, ProviderCall};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::{hex, hex::FromHexError, keccak256, Address, Signature, SignatureError};
use alloy_rpc_client::RpcCall;
use alloy_signer::Signer;
use alloy_transport::{TransportErrorKind, TransportResult};
use http::{HeaderMap, HeaderName, HeaderValue};
use std::future::IntoFuture;

/// Error returned by [`verify_flashbots_signature`].
#[derive(Debug, thiserror::Error)]
pub enum FlashbotsSignatureError {
    /// Invalid signature format, expected `address:signature`.
    #[error("invalid signature format, expected `address:signature`")]
    InvalidFormat,
    /// Invalid address.
    #[error("invalid address")]
    InvalidAddress(#[from] FromHexError),
    /// Invalid signature.
    #[error("invalid signature")]
    InvalidSignature(#[from] SignatureError),
    /// Signature mismatch.
    #[error("signature mismatch: expected {expected}, actual {actual}")]
    SignatureMismatch {
        /// Expected address from the signature header.
        expected: Address,
        /// Actual address recovered from the signature.
        actual: Address,
    },
}

/// A builder for MEV RPC calls that allow optional Flashbots authentication.
pub struct MevBuilder<Params, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: RpcCall<Params, Resp, Output, Map>,
    signer: Option<Box<dyn Signer + Send + Sync>>,
}

impl<Params, Resp, Output, Map> std::fmt::Debug for MevBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend + std::fmt::Debug,
    Resp: RpcRecv + std::fmt::Debug,
    Output: std::fmt::Debug,
    Map: Fn(Resp) -> Output + Clone + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MevBuilder").field("inner", &self.inner).finish()
    }
}

impl<Params, Resp, Output, Map> MevBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    /// Create a new [`MevBuilder`] from a [`RpcCall`].
    pub const fn new_rpc(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self { inner, signer: None }
    }
}

impl<Params, Resp, Output, Map> From<RpcCall<Params, Resp, Output, Map>>
    for MevBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output + Clone,
{
    fn from(inner: RpcCall<Params, Resp, Output, Map>) -> Self {
        Self::new_rpc(inner)
    }
}

impl<Params, Resp, Output, Map> MevBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Enables Flashbots authentication using the provided signer.
    ///
    /// The signer is used to generate the `X-Flashbots-Signature` header, which will be included
    /// in the request if the transport supports HTTP headers.
    pub fn with_auth<S: Signer + Send + Sync + 'static>(mut self, signer: S) -> Self {
        self.signer = Some(Box::new(signer));
        self
    }
}

impl<Params, Resp, Output, Map> IntoFuture for MevBuilder<Params, Resp, Output, Map>
where
    Params: RpcSend + 'static,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output + Send + 'static,
{
    type Output = TransportResult<Output>;
    type IntoFuture = ProviderCall<Params, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        if let Some(signer) = self.signer {
            let fut = async move {
                // Generate the Flashbots signature for the request body
                let body = serde_json::to_string(&self.inner.request())
                    .map_err(TransportErrorKind::custom)?;
                let signature = sign_flashbots_payload(body, &signer)
                    .await
                    .map_err(TransportErrorKind::custom)?;

                // Add the Flashbots signature to the request headers
                let headers = HeaderMap::from_iter([(
                    HeaderName::from_static(FLASHBOTS_SIGNATURE_HEADER),
                    HeaderValue::from_str(signature.as_str())
                        .map_err(TransportErrorKind::custom)?,
                )]);

                // Patch the existing RPC call with the new headers
                let rpc_call = self.inner.map_meta(|mut meta| {
                    meta.extensions_mut().get_or_insert_default::<HeaderMap>().extend(headers);
                    meta
                });

                rpc_call.await
            };
            return ProviderCall::BoxedFuture(Box::pin(fut));
        }
        ProviderCall::RpcCall(self.inner)
    }
}

/// Uses the provided signer to generate a Flashbots `X-Flashbots-Signature` header value in the
/// format `{address}:{signature_hex}`.
///
/// See [Flashbots docs](https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#authentication) for more information.
///
/// # Example
///
/// ```
/// use alloy_provider::ext::sign_flashbots_payload;
/// use alloy_signer_local::PrivateKeySigner;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let signer: PrivateKeySigner =
///     "0x0000000000000000000000000000000000000000000000000000000000123456".parse()?;
/// let body = "sign this message".to_string();
/// let signature = sign_flashbots_payload(body, &signer).await?;
/// assert!(signature.starts_with("0xd5F5175D014F28c85F7D67A111C2c9335D7CD771:0x"));
/// # Ok(())
/// # }
/// ```
pub async fn sign_flashbots_payload<S: Signer + Send + Sync>(
    body: String,
    signer: &S,
) -> Result<String, alloy_signer::Error> {
    let message_hash = keccak256(body.as_bytes()).to_string();
    let signature = signer.sign_message(message_hash.as_bytes()).await?;

    // Normalized recovery byte (0/1) following the canonical signature encoding
    let mut sig_bytes = [0u8; 65];
    sig_bytes[..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
    sig_bytes[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
    sig_bytes[64] = signature.v() as u8;
    Ok(format!("{}:{}", signer.address(), hex::encode_prefixed(sig_bytes)))
}

/// Verifies a Flashbots signature and returns the recovered signer address.
///
/// The signature format is `{address}:{signature_hex}` as produced by
/// [`sign_flashbots_payload`]. Both normalized (v=0/1) and EIP-155 (v=27/28)
/// recovery bytes are supported.
///
/// See [Flashbots docs](https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#authentication) for more information.
///
/// # Example
///
/// ```
/// use alloy_provider::ext::verify_flashbots_signature;
///
/// let signature_header = "0xd5F5175D014F28c85F7D67A111C2c9335D7CD771:0x983dc7c520db0d287faff3cd0aef81d5a7f4ffd3473440d3f705da16299724271f660b6fe367f455b205bc014eff3e20defd011f92000f94d39365ca0bc7867200";
/// let body = b"sign this message";
/// let address = verify_flashbots_signature(signature_header, body).unwrap();
/// assert_eq!(address.to_string(), "0xd5F5175D014F28c85F7D67A111C2c9335D7CD771");
/// ```
pub fn verify_flashbots_signature(
    signature_header: &str,
    body: &[u8],
) -> Result<Address, FlashbotsSignatureError> {
    let (address_str, sig_str) =
        signature_header.split_once(':').ok_or(FlashbotsSignatureError::InvalidFormat)?;

    let expected = address_str.parse::<Address>()?;
    let signature = sig_str.parse::<Signature>()?;

    let message_hash = keccak256(body).to_string();
    let actual = signature.recover_address_from_msg(message_hash.as_bytes())?;

    if actual != expected {
        return Err(FlashbotsSignatureError::SignatureMismatch { expected, actual });
    }

    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use alloy_signer_local::PrivateKeySigner;

    const TEST_BODY: &str = "sign this message";
    const TEST_SIGNATURE: &str = "0xd5F5175D014F28c85F7D67A111C2c9335D7CD771:0x983dc7c520db0d287faff3cd0aef81d5a7f4ffd3473440d3f705da16299724271f660b6fe367f455b205bc014eff3e20defd011f92000f94d39365ca0bc7867200";

    #[tokio::test]
    async fn test_sign_flashbots_payload() {
        let signer = PrivateKeySigner::from_bytes(&b256!(
            "0x0000000000000000000000000000000000000000000000000000000000123456"
        ))
        .unwrap();
        let signature = sign_flashbots_payload(TEST_BODY.to_string(), &signer).await.unwrap();
        assert_eq!(signature, TEST_SIGNATURE);
    }

    #[tokio::test]
    async fn test_verify_flashbots_signature_roundtrip() {
        let signer = PrivateKeySigner::from_bytes(&b256!(
            "0x0000000000000000000000000000000000000000000000000000000000123456"
        ))
        .unwrap();

        let signature = sign_flashbots_payload(TEST_BODY.to_string(), &signer).await.unwrap();
        let recovered = verify_flashbots_signature(&signature, TEST_BODY.as_bytes()).unwrap();
        assert_eq!(recovered, signer.address());
    }

    #[test]
    fn test_verify_flashbots_signature_v0() {
        // TEST_SIGNATURE uses v=0 (ends with "00")
        let recovered = verify_flashbots_signature(TEST_SIGNATURE, TEST_BODY.as_bytes()).unwrap();
        assert_eq!(recovered, address!("0xd5F5175D014F28c85F7D67A111C2c9335D7CD771"));
    }

    #[test]
    fn test_verify_flashbots_signature_v27() {
        // Replace last byte: v=0 (00) -> v=27 (1b)
        let signature_v27 = format!("{}1b", &TEST_SIGNATURE[..TEST_SIGNATURE.len() - 2]);
        let recovered = verify_flashbots_signature(&signature_v27, TEST_BODY.as_bytes()).unwrap();
        assert_eq!(recovered, address!("0xd5F5175D014F28c85F7D67A111C2c9335D7CD771"));
    }

    #[test]
    fn test_verify_flashbots_signature_invalid_format() {
        let result = verify_flashbots_signature("invalid", b"body");
        assert!(matches!(result, Err(FlashbotsSignatureError::InvalidFormat)));
    }

    #[test]
    fn test_verify_flashbots_signature_invalid_address() {
        let result = verify_flashbots_signature("notanaddress:0x1234", b"body");
        assert!(matches!(result, Err(FlashbotsSignatureError::InvalidAddress(_))));
    }

    #[test]
    fn test_verify_flashbots_signature_invalid_signature() {
        let result = verify_flashbots_signature(
            "0xd5F5175D014F28c85F7D67A111C2c9335D7CD771:0xinvalid",
            b"body",
        );
        assert!(matches!(result, Err(FlashbotsSignatureError::InvalidSignature(_))));
    }

    #[test]
    fn test_verify_flashbots_signature_mismatch_wrong_address() {
        let wrong_address = Address::repeat_byte(0x01);
        let sig_part = TEST_SIGNATURE.split_once(':').unwrap().1;
        let mismatched = format!("{wrong_address}:{sig_part}");
        let result = verify_flashbots_signature(&mismatched, TEST_BODY.as_bytes());
        assert!(matches!(result, Err(FlashbotsSignatureError::SignatureMismatch { .. })));
    }

    #[test]
    fn test_verify_flashbots_signature_mismatch_wrong_body() {
        let result = verify_flashbots_signature(TEST_SIGNATURE, b"wrong body");
        assert!(matches!(result, Err(FlashbotsSignatureError::SignatureMismatch { .. })));
    }
}
