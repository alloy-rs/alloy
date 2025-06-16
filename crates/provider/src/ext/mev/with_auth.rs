use crate::{ext::FLASHBOTS_SIGNATURE_HEADER, ProviderCall};
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_primitives::{hex, keccak256};
use alloy_rpc_client::RpcCall;
use alloy_signer::Signer;
use alloy_transport::{TransportErrorKind, TransportResult};
use http::{HeaderMap, HeaderName, HeaderValue};
use std::future::IntoFuture;

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
                let rpc_call = self
                    .inner
                    .map_meta(|meta| {
                        let mut meta = meta;
                        meta.extensions_mut().insert(headers);
                        meta
                    })
                    .map_err(TransportErrorKind::custom)?;

                rpc_call.await
            };
            return ProviderCall::BoxedFuture(Box::pin(fut));
        }
        ProviderCall::RpcCall(self.inner)
    }
}

/// Uses the provided signer to generate a signature for Flashbots authentication.
/// Returns the value for the `X-Flashbots-Signature` header.
///
/// See [here](https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#authentication) for more information.
pub async fn sign_flashbots_payload<S: Signer + Send + Sync>(
    body: String,
    signer: &S,
) -> Result<String, alloy_signer::Error> {
    let message_hash = keccak256(body.as_bytes()).to_string();
    let signature = signer.sign_message(message_hash.as_bytes()).await?;
    Ok(format!("{}:{}", signer.address(), hex::encode_prefixed(signature.as_bytes())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::b256;
    use alloy_signer_local::PrivateKeySigner;

    #[tokio::test]
    async fn test_sign_flashbots_payload() {
        let signer = PrivateKeySigner::from_bytes(&b256!(
            "0x0000000000000000000000000000000000000000000000000000000000123456"
        ))
        .unwrap();
        let body = "sign this message".to_string();
        let signature = sign_flashbots_payload(body.clone(), &signer).await.unwrap();
        assert_eq!(signature, "0xd5F5175D014F28c85F7D67A111C2c9335D7CD771:0x983dc7c520db0d287faff3cd0aef81d5a7f4ffd3473440d3f705da16299724271f660b6fe367f455b205bc014eff3e20defd011f92000f94d39365ca0bc786721b");
    }
}
