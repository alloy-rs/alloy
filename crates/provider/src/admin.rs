//! This module extends the Ethereum JSON-RPC provider with the Admin namespace's RPC methods.
use crate::Provider;
use alloy_network::Network;
use alloy_rpc_types::admin::{NodeInfo, PeerInfo};
use alloy_transport::{Transport, TransportResult};

/// Admin namespace rpc interface that gives access to several non-standard RPC methods.
#[allow(unused, unreachable_pub)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait AdminApi<N, T>: Send + Sync {
    /// Requests adding the given peer, returning a boolean representing
    /// whether or not the peer was accepted for tracking.
    async fn add_peer(&self, record: &str) -> TransportResult<bool>;

    /// Requests adding the given peer as a trusted peer, which the node will
    /// always connect to even when its peer slots are full.
    async fn add_trusted_peer(&self, record: &str) -> TransportResult<bool>;

    /// Requests to remove the given peer, returning true if the enode was successfully parsed and
    /// the peer was removed.
    async fn remove_peer(&self, record: &str) -> TransportResult<bool>;

    /// Requests to remove the given peer, returning a boolean representing whether or not the
    /// enode url passed was validated. A return value of `true` does not necessarily mean that the
    /// peer was disconnected.
    async fn remove_trusted_peer(&self, record: &str) -> TransportResult<bool>;

    /// Returns the list of peers currently connected to the node.
    async fn peers(&self) -> TransportResult<Vec<PeerInfo>>;

    /// Returns general information about the node as well as information about the running p2p
    /// protocols (e.g. `eth`, `snap`).
    async fn node_info(&self) -> TransportResult<NodeInfo>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> AdminApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn add_peer(&self, record: &str) -> TransportResult<bool> {
        // self.client().
        self.client().request("admin_addPeer", (record,)).await
    }

    async fn add_trusted_peer(&self, record: &str) -> TransportResult<bool> {
        self.client().request("admin_addTrustedPeer", (record,)).await
    }

    async fn remove_peer(&self, record: &str) -> TransportResult<bool> {
        self.client().request("admin_removePeer", (record,)).await
    }

    async fn remove_trusted_peer(&self, record: &str) -> TransportResult<bool> {
        self.client().request("admin_removeTrustedPeer", (record,)).await
    }

    async fn peers(&self) -> TransportResult<Vec<PeerInfo>> {
        self.client().request("admin_peers", ()).await
    }

    async fn node_info(&self) -> TransportResult<NodeInfo> {
        self.client().request("admin_nodeInfo", ()).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_node_bindings::Geth;

    extern crate self as alloy_provider;

    // NOTE: We cannot import the test-utils crate here due to a circular dependency.
    include!("../../internal-test-utils/src/providers.rs");

    #[tokio::test]
    async fn node_info() {
        let temp_dir = tempfile::TempDir::with_prefix("reth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = http_provider(&geth.endpoint());
        let node_info = provider.node_info().await.unwrap();
        assert!(node_info.enode.starts_with("enode://"));
    }

    #[tokio::test]
    async fn admin_peers() {
        let temp_dir = tempfile::TempDir::with_prefix("reth-test-1").unwrap();
        let temp_dir_2 = tempfile::TempDir::with_prefix("reth-test-2").unwrap();
        let geth1 = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let mut geth2 =
            Geth::new().disable_discovery().port(0u16).data_dir(temp_dir_2.path()).spawn();

        let provider1 = http_provider(&geth1.endpoint());
        let provider2 = http_provider(&geth2.endpoint());
        let node1_info = provider1.node_info().await.unwrap();
        let node1_id = node1_info.id;
        let node1_enode = node1_info.enode;

        let added = provider2.add_peer(&node1_enode).await.unwrap();
        assert!(added);
        geth2.wait_to_add_peer(node1_id).unwrap();
        let peers = provider2.peers().await.unwrap();
        assert_eq!(peers[0].enode, node1_enode);
    }
}
