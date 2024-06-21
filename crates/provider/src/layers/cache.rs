use crate::{Provider, ProviderLayer, RootProvider};
use alloy_json_rpc::{Request, RpcParam};
use alloy_network::Ethereum;
use alloy_primitives::B256;
use alloy_rpc_types_eth::{Block, BlockNumberOrTag};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use lru::LruCache;
use std::{marker::PhantomData, num::NonZeroUsize, path::PathBuf};
use tokio::sync::Mutex;

// TODO: Populate load cache from file on initialization.
// TODO: Add method to dump cache to file.
/// A provider layer that caches RPC responses and serves them on subsequent requests.
#[derive(Debug, Clone)]
pub struct CacheLayer {
    path: PathBuf,
    max_items: usize,
}

impl CacheLayer {
    /// Instantiate a new cache layer.
    pub const fn new(path: PathBuf, max_items: usize) -> Self {
        Self { path, max_items }
    }
}

impl<P, T> ProviderLayer<P, T, Ethereum> for CacheLayer
where
    P: Provider<T>,
    T: Transport + Clone,
{
    type Provider = CacheProvider<P, T>;

    fn layer(&self, inner: P) -> Self::Provider {
        CacheProvider::new(inner, self.path.clone(), self.max_items)
    }
}

/// A provider that caches responses to RPC requests.
#[derive(Debug)]
pub struct CacheProvider<P, T> {
    /// Inner provider.
    inner: P,
    /// In-memory LRU cache, mapping requests to responses.
    cache: Mutex<LruCache<B256, String>>,
    /// Path to the cache file.
    path: PathBuf,
    /// Phantom data
    _pd: PhantomData<T>,
}

impl<P, T> CacheProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    /// Instantiate a new cache provider.
    pub fn new(inner: P, path: PathBuf, max_items: usize) -> Self {
        let cache =
            Mutex::new(LruCache::<B256, String>::new(NonZeroUsize::new(max_items).unwrap()));
        Self { inner, path, cache, _pd: PhantomData }
    }

    /// `keccak256` hash the request params.
    pub fn hash_request<Params: RpcParam>(&self, req: Request<Params>) -> TransportResult<B256> {
        let ser_req = req.serialize().map_err(TransportErrorKind::custom)?;

        Ok(ser_req.params_hash())
    }

    /// Gets the path to the cache file.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Puts a value into the cache, and returns the old value if it existed.
    pub async fn put(&self, key: B256, value: String) -> TransportResult<Option<String>> {
        let mut cache = self.cache.lock().await;
        Ok(cache.put(key, value))
    }

    /// Gets a value from the cache, if it exists.
    pub async fn get(&self, key: &B256) -> TransportResult<Option<String>> {
        let mut cache = self.cache.lock().await;
        let val = cache.get(key).cloned();
        Ok(val)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<P, T> Provider<T> for CacheProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    #[inline(always)]
    fn root(&self) -> &RootProvider<T> {
        self.inner.root()
    }

    async fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        hydrate: bool,
    ) -> TransportResult<Option<Block>> {
        let req = self.inner.client().make_request("eth_getBlockByNumber", (number, hydrate));

        let hash = self.hash_request(req)?;

        // Try to get from cache
        if let Some(block) = self.get(&hash).await? {
            println!("Returned from cache!");
            let block = serde_json::from_str(&block).map_err(TransportErrorKind::custom)?;
            return Ok(Some(block));
        }

        let rpc_res = self.inner.get_block_by_number(number, hydrate).await?;
        println!("Returned from RPC!");
        if let Some(ref block) = rpc_res {
            let json_str = serde_json::to_string(block).map_err(TransportErrorKind::custom)?;
            let old_val = self.put(hash, json_str).await?;
        }

        Ok(rpc_res)
    }

    // TODO: Add other commonly used methods such as eth_getTransactionByHash, eth_getProof,
    // eth_getStorage etc.
}

#[cfg(test)]
mod tests {
    use crate::ProviderBuilder;
    use alloy_node_bindings::Anvil;

    use super::*;

    #[tokio::test]
    async fn test_cache_provider() {
        let cache = CacheLayer::new(PathBuf::new(), 100);
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());

        let blk = provider.get_block_by_number(0.into(), true).await.unwrap();
        let blk2 = provider.get_block_by_number(0.into(), true).await.unwrap();

        assert_eq!(blk, blk2);
    }
}
