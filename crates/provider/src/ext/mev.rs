use crate::Provider;
use alloy_json_rpc::{HttpHeaderExtension, Request};
use alloy_network::Network;
use alloy_primitives::{hex, keccak256};
use alloy_rpc_client::RpcCall;
use alloy_rpc_types_mev::{EthBundleHash, EthSendBundle};
use alloy_signer::Signer;
use alloy_transport::{TransportErrorKind, TransportResult};
use std::borrow::Cow;

/// The HTTP header used for Flashbots signature authentication.
pub const FLASHBOTS_SIGNATURE_HEADER: &str = "X-Flashbots-Signature";

/// This module provides support for interacting with non-standard MEV-related RPC endpoints.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait MevApi<N>: Send + Sync {
    /// Sends a MEV bundle using the `eth_sendBundle` RPC method.
    /// Returns the resulting bundle hash on success.
    async fn send_bundle(&self, bundle: EthSendBundle) -> TransportResult<EthBundleHash>;

    /// Sends a MEV bundle via the `eth_sendBundle` RPC method with `X-Flashbots-Signature`
    /// authentication header. The provided signer is used to generate the signature.
    /// Returns the resulting bundle hash on success.
    async fn send_bundle_with_auth<S: Signer + Send + Sync>(
        &self,
        bundle: EthSendBundle,
        signer: S,
    ) -> TransportResult<EthBundleHash>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> MevApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn send_bundle(&self, bundle: EthSendBundle) -> TransportResult<EthBundleHash> {
        self.client().request("eth_sendBundle", (bundle,)).await
    }

    async fn send_bundle_with_auth<S: Signer + Send + Sync>(
        &self,
        bundle: EthSendBundle,
        signer: S,
    ) -> TransportResult<EthBundleHash> {
        let mut request = Request::<Vec<EthSendBundle>>::new(
            Cow::Borrowed("eth_sendBundle"),
            0.into(),
            vec![bundle.clone()],
        );
        // Generate the Flashbots signature for the request body
        let body = serde_json::to_string(&request).map_err(TransportErrorKind::custom)?;
        let signature =
            sign_flashbots_payload(body, &signer).await.map_err(TransportErrorKind::custom)?;

        let headers: HttpHeaderExtension =
            HttpHeaderExtension::from_iter([(FLASHBOTS_SIGNATURE_HEADER.to_string(), signature)]);

        request.meta.extensions_mut().insert(headers);
        RpcCall::new(request, self.client().transport().clone()).await
    }
}

/// Uses the provided signer to generate a signature for Flashbots authentication.
/// Returns the value for the `X-Flashbots-Signature` header.
///
/// See [here](https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#authentication) for more information.
pub async fn sign_flashbots_payload<S: Signer + Sync>(
    body: String,
    signer: &S,
) -> Result<String, alloy_signer::Error> {
    let message_hash = keccak256(body.as_bytes()).to_string();
    let signature = signer.sign_message(message_hash.as_bytes()).await?;
    Ok(format!("{}:{}", signer.address(), hex::encode_prefixed(signature.as_bytes())))
}

#[cfg(test)]
mod tests {
    use crate::ext::mev::sign_flashbots_payload;
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
