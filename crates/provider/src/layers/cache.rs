use crate::{Provider, ProviderLayer, RootProvider};
use alloy_json_rpc::{Request, RpcParam};
use alloy_network::Ethereum;
use alloy_primitives::B256;
use alloy_rpc_types_eth::{Block, BlockNumberOrTag};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{io::BufReader, marker::PhantomData, num::NonZeroUsize, path::PathBuf};
use tokio::sync::RwLock;
// TODO: Populate load cache from file on initialization.
// TODO: Add method to dump cache to file.
/// A provider layer that caches RPC responses and serves them on subsequent requests.
///
/// In order to initialize the caching layer, the path to the cache file is provided along with the
/// max number of items that are stored in the in-memory LRU cache.
///
/// One can load the cache from the file system by calling `load_cache` and save the cache to the
/// file system by calling `save_cache`.
///
/// Example usage:
/// ```
/// use alloy_node_bindings::Anvil;
/// use alloy_provider::ProviderBuilder;
/// use std::path::PathBuf;
///
/// let cache = CacheLayer::new(100);
/// let anvil = Anvil::new().block_time_f64(0.3).spawn();
/// let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());
/// let path = PathBuf::from_str("./rpc-cache.txt").unwrap();
/// provider.load_cache(path).await.unwrap(); // Load cache from file if it exists.
///
/// let blk = provider.get_block_by_number(0.into(), true).await.unwrap(); // Fetched from RPC and saved to in-memory cache
///
/// let blk2 = provider.get_block_by_number(0.into(), true).await.unwrap(); // Fetched from in-memory cache
/// assert_eq!(blk, blk2);
///
/// provider.save_cache(path).await.unwrap(); // Save cache to file
/// ```
#[derive(Debug, Clone)]
pub struct CacheLayer {
    config: CacheConfig,
}

impl CacheLayer {
    /// Instantiate a new cache layer with the the maximum number of
    /// items to store.
    #[inline]
    pub const fn new(max_items: usize) -> Self {
        Self { config: CacheConfig { max_items } }
    }

    /// Returns the maximum number of items that can be stored in the cache, set at initialization.
    #[inline]
    pub const fn max_items(&self) -> usize {
        self.config.max_items
    }
}

impl<P, T> ProviderLayer<P, T, Ethereum> for CacheLayer
where
    P: Provider<T>,
    T: Transport + Clone,
{
    type Provider = CacheProvider<P, T>;

    fn layer(&self, inner: P) -> Self::Provider {
        CacheProvider::new(inner, self.max_items())
    }
}

/// A provider that caches responses to RPC requests.
#[derive(Debug)]
pub struct CacheProvider<P, T> {
    /// Inner provider.
    inner: P,
    /// In-memory LRU cache, mapping requests to responses.
    cache: RwLock<LruCache<B256, String>>,
    /// Phantom data
    _pd: PhantomData<T>,
}

impl<P, T> CacheProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    /// Instantiate a new cache provider.
    pub fn new(inner: P, max_items: usize) -> Self {
        let cache =
            RwLock::new(LruCache::<B256, String>::new(NonZeroUsize::new(max_items).unwrap()));
        let provider = Self { inner, cache, _pd: PhantomData };
        provider
    }

    /// `keccak256` hash the request params.
    pub fn hash_request<Params: RpcParam>(&self, req: Request<Params>) -> TransportResult<B256> {
        let ser_req = req.serialize().map_err(TransportErrorKind::custom)?;

        Ok(ser_req.params_hash())
    }

    /// Puts a value into the cache, and returns the old value if it existed.
    pub async fn put(&self, key: B256, value: String) -> TransportResult<Option<String>> {
        let mut cache = self.cache.write().await;
        Ok(cache.put(key, value))
    }

    /// Gets a value from the cache, if it exists.
    pub async fn get(&self, key: &B256) -> TransportResult<Option<String>> {
        // Need to acquire a write guard to change the order of keys in LRU cache.
        let mut cache = self.cache.write().await;
        let val = cache.get(key).cloned();
        Ok(val)
    }

    /// Saves the cache to a file specified by the path.
    /// If the files does not exist, it creates one.
    /// If the file exists, it overwrites it.
    pub async fn save_cache(&self, path: PathBuf) -> TransportResult<()> {
        let cache = self.cache.read().await;
        let file = std::fs::File::create(path).map_err(TransportErrorKind::custom)?;

        // Iterate over the cache and dump to the file.
        let entries = cache
            .iter()
            .map(|(key, value)| FsCacheEntry { key: *key, value: value.clone() })
            .collect::<Vec<_>>();
        serde_json::to_writer(file, &entries).map_err(TransportErrorKind::custom)?;
        Ok(())
    }

    /// Loads the cache from a file specified by the path.
    /// If the file does not exist, it returns without error.
    pub async fn load_cache(&self, path: PathBuf) -> TransportResult<()> {
        if !path.exists() {
            return Ok(());
        };
        let file = std::fs::File::open(path).map_err(TransportErrorKind::custom)?;
        let file = BufReader::new(file);
        let entries: Vec<FsCacheEntry> =
            serde_json::from_reader(file).map_err(TransportErrorKind::custom)?;
        let mut cache = self.cache.write().await;
        for entry in entries {
            cache.put(entry.key, entry.value);
        }

        println!("Loaded {} cache entries", cache.len());
        Ok(())
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
            let _old_val = self.put(hash, json_str).await?;
        }

        Ok(rpc_res)
    }

    // TODO: Add other commonly used methods such as eth_getTransactionByHash, eth_getProof,
    // eth_getStorage etc.
}

#[derive(Debug, Serialize, Deserialize)]
struct FsCacheEntry {
    /// Hash of the request params
    key: B256,
    /// Serialized response to the request from which the hash was computed.
    value: String,
}

/// Configuration for the cache layer.
/// For future extensibility of the configurations.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of items to store in the cache.
    pub max_items: usize,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::ProviderBuilder;
    use alloy_node_bindings::Anvil;

    use super::*;

    #[tokio::test]
    async fn test_cache_provider() {
        let cache = CacheLayer::new(100);
        let anvil = Anvil::new().block_time_f64(0.3).spawn();
        let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());

        let path = PathBuf::from_str("./rpc-cache.txt").unwrap();
        provider.load_cache(path.clone()).await.unwrap();
        let blk = provider.get_block_by_number(0.into(), true).await.unwrap();
        let blk2 = provider.get_block_by_number(0.into(), true).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let latest_block_num = provider.get_block_number().await.unwrap();
        let blk3 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();
        let blk4 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();

        provider.save_cache(path).await.unwrap();
        assert_eq!(blk, blk2);
        assert_eq!(blk3, blk4);
    }
}
