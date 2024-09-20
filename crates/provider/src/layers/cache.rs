use crate::{Provider, ProviderLayer, RootProvider, RpcWithBlock};
use alloy_json_rpc::{Request, RpcParam};
use alloy_network::Ethereum;
use alloy_primitives::{Address, BlockHash, StorageKey, StorageValue, B256, U256};
use alloy_rpc_client::ClientRef;
use alloy_rpc_types_eth::{
    Block, BlockId, BlockNumberOrTag, BlockTransactionsKind, EIP1186AccountProofResponse,
};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{io::BufReader, marker::PhantomData, num::NonZeroUsize, path::PathBuf, sync::Arc};
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
#[derive(Debug, Clone)]
pub struct CacheProvider<P, T> {
    /// Inner provider.
    inner: P,
    /// In-memory LRU cache, mapping requests to responses.
    cache: Arc<RwLock<LruCache<B256, String>>>,
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
        let cache = Arc::new(RwLock::new(LruCache::<B256, String>::new(
            NonZeroUsize::new(max_items).unwrap(),
        )));
        Self { inner, cache, _pd: PhantomData }
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
        let hash =
            RequestType::GetBlockByNumber((number, hydrate)).params_hash(self.inner.client())?;

        if let Some(block) = self.get(&hash).await? {
            let block = serde_json::from_str(&block).map_err(TransportErrorKind::custom)?;
            println!("Cache hit");
            return Ok(Some(block));
        }

        println!("Cache miss");
        let block = self.inner.get_block_by_number(number, hydrate).await?;
        if let Some(ref block) = block {
            let json_str = serde_json::to_string(block).map_err(TransportErrorKind::custom)?;
            let _ = self.put(hash, json_str).await?;
        }

        Ok(block)
    }

    /// Gets a block by its [BlockHash], with full transactions or only hashes.
    async fn get_block_by_hash(
        &self,
        hash: BlockHash,
        kind: BlockTransactionsKind,
    ) -> TransportResult<Option<Block>> {
        let full = match kind {
            BlockTransactionsKind::Full => true,
            BlockTransactionsKind::Hashes => false,
        };

        let req_hash =
            RequestType::GetBlockByHash((hash, full)).params_hash(self.inner.client())?;

        if let Some(block) = self.get(&req_hash).await? {
            let block = serde_json::from_str(&block).map_err(TransportErrorKind::custom)?;
            println!("Cache hit");
            return Ok(Some(block));
        }

        println!("Cache miss");
        let block = self.inner.get_block_by_hash(hash, kind).await?;
        if let Some(ref block) = block {
            let json_str = serde_json::to_string(block).map_err(TransportErrorKind::custom)?;
            let _ = self.put(req_hash, json_str).await?;
        }

        Ok(block)
    }

    // TODO: Add other commonly used methods such as eth_getTransactionByHash
    // TODO: Any methods returning RpcWithBlock or RpcCall are blocked by https://github.com/alloy-rs/alloy/pull/788

    /// Get the account and storage values of the specified account including the merkle proofs.
    ///
    /// This call can be used to verify that the data has not been tampered with.
    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<T, (Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        todo!()
        // Blocked by https://github.com/alloy-rs/alloy/pull/788
    }

    /// Gets the specified storage value from [Address].
    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> RpcWithBlock<T, (Address, U256), StorageValue> {
        todo!()
        // Blocked by https://github.com/alloy-rs/alloy/pull/788
    }
}

/// Enum representing different RPC requests.
///
/// Useful for handling hashing of various request parameters.
enum RequestType<Params: RpcParam> {
    /// Get block by number.
    GetBlockByNumber(Params),
    /// Get block by hash.
    GetBlockByHash(Params),
    /// Get proof.
    GetProof(Params),
    /// Get storage at.
    GetStorageAt(Params),
}

impl<Params: RpcParam> RequestType<Params> {
    fn make_request<T: Transport>(&self, client: ClientRef<'_, T>) -> Request<Params> {
        let (method, params) = match self {
            Self::GetBlockByNumber(params) => ("eth_getBlockByNumber", params),
            Self::GetBlockByHash(params) => ("eth_getBlockByHash", params),
            Self::GetProof(params) => ("eth_getProof", params),
            Self::GetStorageAt(params) => ("eth_getStorageAt", params),
        };
        client.make_request(method, params.to_owned())
    }

    /// `keccak256` hash the request params.
    fn params_hash<T: Transport>(&self, client: ClientRef<'_, T>) -> TransportResult<B256> {
        let req = self.make_request(client);
        let ser_req = req.serialize().map_err(TransportErrorKind::custom)?;

        Ok(ser_req.params_hash())
    }
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
        assert_eq!(blk, blk2);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let latest_block_num = provider.get_block_number().await.unwrap();
        let blk3 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();
        let blk4 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();
        assert_eq!(blk3, blk4);

        provider.save_cache(path).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_block() {
        let cache = CacheLayer::new(100);
        let anvil = Anvil::new().block_time_f64(0.3).spawn();
        let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());

        let path = PathBuf::from_str("./rpc-cache.txt").unwrap();
        provider.load_cache(path.clone()).await.unwrap();

        let block = provider.get_block(0.into(), BlockTransactionsKind::Full).await.unwrap(); // Received from RPC.
        let block2 = provider.get_block(0.into(), BlockTransactionsKind::Full).await.unwrap(); // Received from cache.
        assert_eq!(block, block2);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let latest_block =
            provider.get_block(BlockId::latest(), BlockTransactionsKind::Full).await.unwrap(); // Received from RPC.
        let latest_hash = latest_block.unwrap().header.hash;

        let block3 =
            provider.get_block_by_hash(latest_hash, BlockTransactionsKind::Full).await.unwrap(); // Received from RPC.
        let block4 =
            provider.get_block_by_hash(latest_hash, BlockTransactionsKind::Full).await.unwrap(); // Received from cache.
        assert_eq!(block3, block4);

        provider.save_cache(path).await.unwrap();
    }
}
