mod with_auth;

pub use self::with_auth::{sign_flashbots_payload, MevBuilder};
use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{hex, TxHash};
use alloy_rpc_types_mev::{
    EthBundleHash, EthCallBundle, EthCallBundleResponse, EthCancelBundle,
    EthCancelPrivateTransaction, EthSendBlobs, EthSendBundle, EthSendEndOfBlockBundle,
    EthSendPrivateTransaction, MevSendBundle, PrivateTransactionPreferences,
};

/// The HTTP header used for Flashbots signature authentication.
pub const FLASHBOTS_SIGNATURE_HEADER: &str = "x-flashbots-signature";

/// This module provides support for interacting with non-standard MEV-related RPC endpoints.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait MevApi<N>: Send + Sync {
    /// Simulates a bundle of transactions using the `eth_callBundle` RPC method.
    fn call_bundle(
        &self,
        bundle: EthCallBundle,
    ) -> MevBuilder<(EthCallBundle,), Option<EthCallBundleResponse>>;

    /// Sends a MEV bundle using the `eth_sendBundle` RPC method.
    /// Returns the resulting bundle hash on success.
    fn send_bundle(
        &self,
        bundle: EthSendBundle,
    ) -> MevBuilder<(EthSendBundle,), Option<EthBundleHash>>;

    /// Cancels a previously sent MEV bundle using the `eth_cancelBundle` RPC method.
    fn cancel_bundle(&self, replacement_uuid: String) -> MevBuilder<(EthCancelBundle,), ()>;

    /// Sends blob transaction permutations using the `eth_sendBlobs` RPC method.
    fn send_blobs(&self, blobs: EthSendBlobs) -> MevBuilder<(EthSendBlobs,), ()>;

    /// Sends a private transaction using the `eth_sendPrivateTransaction` RPC method.
    fn send_private_transaction(
        &self,
        private_tx: EthSendPrivateTransaction,
    ) -> MevBuilder<(EthSendPrivateTransaction,), Option<TxHash>>;

    /// Sends a private transaction using the `eth_sendPrivateRawTransaction` RPC method.
    fn send_private_raw_transaction(
        &self,
        encoded_tx: &[u8],
        preferences: Option<PrivateTransactionPreferences>,
    ) -> MevBuilder<(String, Option<PrivateTransactionPreferences>), Option<TxHash>>;

    /// Cancels a previously sent private transaction using the `eth_cancelPrivateTransaction` RPC
    /// method.
    fn cancel_private_transaction(
        &self,
        tx_hash: TxHash,
    ) -> MevBuilder<(EthCancelPrivateTransaction,), bool>;

    /// Sends end-of-block bundle using the `eth_sendEndOfBlockBundle` RPC method.
    /// Returns the resulting bundle hash on success.
    fn send_end_of_block_bundle(
        &self,
        bundle: EthSendEndOfBlockBundle,
    ) -> MevBuilder<(EthSendEndOfBlockBundle,), Option<EthBundleHash>>;

    /// Sends a MEV bundle using the `mev_sendBundle` RPC method.
    /// Returns the resulting bundle hash on success.
    fn send_mev_bundle(
        &self,
        bundle: MevSendBundle,
    ) -> MevBuilder<(MevSendBundle,), Option<EthBundleHash>>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> MevApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    fn call_bundle(
        &self,
        bundle: EthCallBundle,
    ) -> MevBuilder<(EthCallBundle,), Option<EthCallBundleResponse>> {
        MevBuilder::new_rpc(self.client().request("eth_callBundle", (bundle,)))
    }

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

    fn send_blobs(&self, blobs: EthSendBlobs) -> MevBuilder<(EthSendBlobs,), ()> {
        MevBuilder::new_rpc(self.client().request("eth_sendBlobs", (blobs,)))
    }

    fn send_private_transaction(
        &self,
        private_tx: EthSendPrivateTransaction,
    ) -> MevBuilder<(EthSendPrivateTransaction,), Option<TxHash>> {
        MevBuilder::new_rpc(self.client().request("eth_sendPrivateTransaction", (private_tx,)))
    }

    fn send_private_raw_transaction(
        &self,
        encoded_tx: &[u8],
        preferences: Option<PrivateTransactionPreferences>,
    ) -> MevBuilder<(String, Option<PrivateTransactionPreferences>), Option<TxHash>> {
        let rlp_hex = hex::encode_prefixed(encoded_tx);
        MevBuilder::new_rpc(
            self.client().request("eth_sendPrivateRawTransaction", (rlp_hex, preferences)),
        )
    }

    fn cancel_private_transaction(
        &self,
        tx_hash: TxHash,
    ) -> MevBuilder<(EthCancelPrivateTransaction,), bool> {
        MevBuilder::new_rpc(
            self.client().request(
                "eth_cancelPrivateTransaction",
                (EthCancelPrivateTransaction { tx_hash },),
            ),
        )
    }

    fn send_end_of_block_bundle(
        &self,
        bundle: EthSendEndOfBlockBundle,
    ) -> MevBuilder<(EthSendEndOfBlockBundle,), Option<EthBundleHash>> {
        MevBuilder::new_rpc(self.client().request("eth_sendEndOfBlockBundle", (bundle,)))
    }

    fn send_mev_bundle(
        &self,
        bundle: MevSendBundle,
    ) -> MevBuilder<(MevSendBundle,), Option<EthBundleHash>> {
        MevBuilder::new_rpc(self.client().request("mev_sendBundle", (bundle,)))
    }
}
