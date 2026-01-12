//! Reth-specific provider extensions.
#[cfg(feature = "pubsub")]
use crate::GetSubscription;
use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{map::HashMap, Address, U256};
use alloy_rpc_types_eth::BlockId;
use alloy_transport::TransportResult;

/// Reth API namespace for reth-specific methods
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait RethProviderExt<N: Network>: Send + Sync {
    /// Returns all ETH balance changes in a block
    async fn reth_get_balance_changes_in_block(
        &self,
        block_id: BlockId,
    ) -> TransportResult<HashMap<Address, U256>>;

    /// Subscribe to json `ChainNotifications`
    #[cfg(feature = "pubsub")]
    async fn reth_subscribe_chain_notifications(
        &self,
    ) -> GetSubscription<alloy_rpc_client::NoParams, serde_json::Value>;

    /// Subscribe to persisted block notifications.
    ///
    /// Emits a notification with the block number and hash when a new block is persisted to disk.
    #[cfg(feature = "pubsub")]
    async fn reth_subscribe_persisted_block(
        &self,
    ) -> GetSubscription<alloy_rpc_client::NoParams, serde_json::Value>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> RethProviderExt<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn reth_get_balance_changes_in_block(
        &self,
        block_id: BlockId,
    ) -> TransportResult<HashMap<Address, U256>> {
        self.client().request("reth_getBalanceChangesInBlock", (block_id,)).await
    }

    #[cfg(feature = "pubsub")]
    async fn reth_subscribe_chain_notifications(
        &self,
    ) -> GetSubscription<alloy_rpc_client::NoParams, serde_json::Value> {
        self.subscribe_to("reth_subscribeChainNotifications")
    }

    #[cfg(feature = "pubsub")]
    async fn reth_subscribe_persisted_block(
        &self,
    ) -> GetSubscription<alloy_rpc_client::NoParams, serde_json::Value> {
        self.subscribe_to("reth_subscribePersistedBlock")
    }
}
