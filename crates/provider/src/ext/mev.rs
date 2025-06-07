use crate::Provider;
use alloy_json_rpc::Request;
use alloy_network::Network;
use alloy_primitives::{hex, keccak256};
use alloy_rpc_types_mev::{EthBundleHash, EthSendBundle};
use alloy_signer::Signer;
use alloy_transport::{TransportErrorKind, TransportResult};
use std::borrow::Cow;

/// The HTTP header used to send the Flashbots signature for authentication.
pub const FLASHBOTS_SIGNATURE_HEADER: &str = "X-Flashbots-Signature";

/// MEV rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait MevApi<N>: Send + Sync {
    /// Send a bundle to the builder.
    async fn send_bundle(&self, bundle: EthSendBundle) -> TransportResult<EthBundleHash>;

    /// Use the provided signer to create a signature for the bundle and send the bundle with http
    /// header `X-Flashbots-Signature` to the builder.
    async fn send_signed_bundle<S: Signer + Send + Sync>(
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
        #[cfg(feature = "pubsub")]
        self.client().pubsub_frontend().expect("This provider does not support pubsub");

        self.client().request("eth_sendBundle", (bundle,)).await
    }

    async fn send_signed_bundle<S: Signer + Send + Sync>(
        &self,
        bundle: EthSendBundle,
        signer: S,
    ) -> TransportResult<EthBundleHash> {
        #[cfg(feature = "pubsub")]
        self.client().pubsub_frontend().expect("This provider does not support pubsub");

        let req = Request::<Vec<EthSendBundle>>::new(
            Cow::Borrowed("eth_sendBundle"),
            0.into(),
            vec![bundle.clone()],
        );
        let body = serde_json::to_string(&req).map_err(TransportErrorKind::custom)?;
        let _signature = sign_flashbots_payload(body, &signer)
            .await
            .map_err(TransportErrorKind::custom)?
            .as_str();

        // TODO: How to set the header in the request?

        self.client().request("eth_sendBundle", (req,)).await
    }
}

/// Sign the payload with the provided signer for Flashbots authentication. It returns the
/// authentication header value that can be used for `X-Flashbots-Signature`.
///
/// See [here](https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#authentication) for more details.
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
