#[cfg(feature = "pubsub")]
use crate::GetSubscription;
use alloy_json_rpc::RpcError;
use alloy_primitives::{map::HashMap, Address, U256};
use alloy_rpc_types_eth::BlockId;

/// Reth API namespace for reth-specific methods
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait RethProviderExt {
    /// Returns all ETH balance changes in a block
    async fn reth_get_balance_changes_in_block(
        &self,
        block_id: BlockId,
    ) -> Result<HashMap<Address, U256>, RpcError<()>>;

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
