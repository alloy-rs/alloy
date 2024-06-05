//! This modules extends the Ethereum JSON-RPC provider with the `txpool` namespace.
use crate::Provider;
use alloy_network::{Ethereum, Network};
use alloy_primitives::Address;
use alloy_rpc_types::txpool::{TxPoolContent, TxPoolContentFrom, TxPoolInspect, TxPoolStatus};
use alloy_transport::{Transport, TransportResult};

/// TxPool namespace rpc interface.
#[allow(unused, unreachable_pub)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait TxPoolApi<T, N = Ethereum>: Send + Sync {
    /// Returns the content of the transaction pool.
    ///
    /// Lists the exact details of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    async fn txpool_content(&self) -> TransportResult<TxPoolContent>;

    /// Returns the content of the transaction pool filtered by a specific address.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    async fn txpool_content_from(&self, from: Address) -> TransportResult<TxPoolContentFrom>;

    /// Returns a textual summary of each transaction in the pool.
    ///
    /// Lists a textual summary of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    /// This is a method specifically tailored to developers to quickly see the
    /// transactions in the pool and find any potential issues.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_inspect) for more details
    async fn txpool_inspect(&self) -> TransportResult<TxPoolInspect>;

    /// Returns the current status of the transaction pool.
    ///
    /// i.e the number of transactions currently pending for inclusion in the next block(s), as well
    /// as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_status) for more details
    async fn txpool_status(&self) -> TransportResult<TxPoolStatus>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<P, T, N> TxPoolApi<T, N> for P
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    async fn txpool_content(&self) -> TransportResult<TxPoolContent> {
        self.client().request("txpool_content", ()).await
    }

    async fn txpool_content_from(&self, from: Address) -> TransportResult<TxPoolContentFrom> {
        self.client().request("txpool_contentFrom", (from,)).await
    }

    async fn txpool_inspect(&self) -> TransportResult<TxPoolInspect> {
        self.client().request("txpool_inspect", ()).await
    }

    async fn txpool_status(&self) -> TransportResult<TxPoolStatus> {
        self.client().request("txpool_status", ()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ProviderBuilder;

    use super::*;
    use alloy_node_bindings::Geth;

    #[tokio::test]
    async fn test_txpool_content() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());
        let content = provider.txpool_content().await.unwrap();
        assert_eq!(content, TxPoolContent::default());
    }

    #[tokio::test]
    async fn test_txpool_content_from() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());
        let content = provider.txpool_content_from(Address::default()).await.unwrap();
        assert_eq!(content, TxPoolContentFrom::default());
    }

    #[tokio::test]
    async fn test_txpool_inspect() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());
        let content = provider.txpool_inspect().await.unwrap();
        assert_eq!(content, TxPoolInspect::default());
    }

    #[tokio::test]
    async fn test_txpool_status() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());
        let content = provider.txpool_status().await.unwrap();
        assert_eq!(content, TxPoolStatus::default());
    }
}
