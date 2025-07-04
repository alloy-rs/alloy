mod with_auth;

pub use self::with_auth::{sign_flashbots_payload, MevBuilder};
use crate::Provider;
use alloy_network::Network;
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};

/// The HTTP header used for Flashbots signature authentication.
pub const FLASHBOTS_SIGNATURE_HEADER: &str = "x-flashbots-signature";

/// This module provides support for interacting with non-standard MEV-related RPC endpoints.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait MevApi<N>: Send + Sync {
    /// Sends a MEV bundle using the `eth_sendBundle` RPC method.
    /// Returns the resulting bundle hash on success.
    fn send_bundle(
        &self,
        bundle: EthSendBundle,
    ) -> MevBuilder<(EthSendBundle,), Option<EthBundleHash>>;

    /// Cancels a previously sent MEV bundle using the `eth_cancelBundle` RPC method.
    fn cancel_bundle(&self, replacement_uuid: String) -> MevBuilder<(EthCancelBundle,), ()>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> MevApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    fn send_bundle(
        &self,
        bundle: EthSendBundle,
    ) -> MevBuilder<(EthSendBundle,), Option<EthBundleHash>> {
        MevBuilder::new_rpc(self.client().request("eth_sendBundle", (bundle,)))
    }

    fn cancel_bundle(&self, replacement_uuid: String) -> MevBuilder<(EthCancelBundle,), ()> {
        MevBuilder::new_rpc(
            self.client().request("eth_cancelBundle", (EthCancelBundle { replacement_uuid },)),
        )
    }
}
