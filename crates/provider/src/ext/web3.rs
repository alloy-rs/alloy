//! This module extends the Ethereum JSON-RPC provider with the Web3 namespace's RPC methods.
use crate::Provider;
use alloy_network::Network;
use alloy_transport::{Transport, TransportResult};

/// Web3 namespace rpc interface that provides access to web3 information of the node.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Web3Api<N, T>: Send + Sync {
    /// Gets the client version of the chain client.
    async fn web3_client_version(&self) -> TransportResult<String>;
    /// Gets the Keccak-256 hash of the given data.
    async fn web3_sha3(&self, data: &[u8]) -> TransportResult<String>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> Web3Api<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn web3_client_version(&self) -> TransportResult<String> {
        self.client().request("web3_clientVersion", ()).await
    }

    async fn web3_sha3(&self, data: &[u8]) -> TransportResult<String> {
        self.client().request("web3_sha3", (data,)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ProviderBuilder;

    use super::*;

    #[tokio::test]
    async fn test_web3_client_version() {
        let provider = ProviderBuilder::new().on_anvil();
        let version = provider.web3_client_version().await.unwrap();
        assert!(!version.is_empty());
    }

    #[tokio::test]
    async fn test_web3_sha3() {
        let provider = ProviderBuilder::new().on_anvil();
        let data = "alloy".as_bytes();
        let hash = provider.web3_sha3(data).await.unwrap();
        assert!(!hash.is_empty());
    }
}
