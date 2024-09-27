use crate::{ParamsWithBlock, Provider, ProviderCall, ProviderLayer, RootProvider, RpcWithBlock};
use alloy_json_rpc::{RpcError, RpcParam};
use alloy_network::Ethereum;
use alloy_primitives::{keccak256, Address, BlockHash, StorageKey, StorageValue, B256, U256};
use alloy_rpc_types_eth::{
    Block, BlockNumberOrTag, BlockTransactionsKind, EIP1186AccountProofResponse,
};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{io::BufReader, marker::PhantomData, num::NonZeroUsize, path::PathBuf, sync::Arc};

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
/// use alloy_provider::{ProviderBuilder, Provider};
/// use alloy_provider::layers::CacheLayer;
/// use std::path::PathBuf;
/// use std::str::FromStr;
///
/// #[tokio::main]
/// async fn main() {
/// let cache = CacheLayer::new(100);
/// let anvil = Anvil::new().block_time_f64(0.3).spawn();
/// let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());
/// let path = PathBuf::from_str("./rpc-cache.txt").unwrap();
/// provider.load_cache(path.clone()).unwrap(); // Load cache from file if it exists.
///
/// let blk = provider.get_block_by_number(0.into(), true).await.unwrap(); // Fetched from RPC and saved to in-memory cache
///
/// let blk2 = provider.get_block_by_number(0.into(), true).await.unwrap(); // Fetched from in-memory cache
/// assert_eq!(blk, blk2);
///
/// provider.save_cache(path).unwrap(); // Save cache to file
/// }
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
    pub fn put(&self, key: B256, value: String) -> TransportResult<Option<String>> {
        let mut cache = self.cache.write();
        Ok(cache.put(key, value))
    }

    /// Gets a value from the cache, if it exists.
    pub fn get(&self, key: &B256) -> TransportResult<Option<String>> {
        // Need to acquire a write guard to change the order of keys in LRU cache.
        let mut cache = self.cache.write();
        let val = cache.get(key).cloned();
        Ok(val)
    }

    /// Saves the cache to a file specified by the path.
    /// If the files does not exist, it creates one.
    /// If the file exists, it overwrites it.
    pub fn save_cache(&self, path: PathBuf) -> TransportResult<()> {
        let cache = self.cache.read();
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
    pub fn load_cache(&self, path: PathBuf) -> TransportResult<()> {
        if !path.exists() {
            return Ok(());
        };
        let file = std::fs::File::open(path).map_err(TransportErrorKind::custom)?;
        let file = BufReader::new(file);
        let entries: Vec<FsCacheEntry> =
            serde_json::from_reader(file).map_err(TransportErrorKind::custom)?;
        let mut cache = self.cache.write();
        for entry in entries {
            cache.put(entry.key, entry.value);
        }

        Ok(())
    }
}

macro_rules! cache_get_or_fetch {
    ($self:expr, $hash:expr, $fetch_fn:expr) => {{
        if let Some(cached) = $self.get(&$hash)? {
            let result = serde_json::from_str(&cached).map_err(TransportErrorKind::custom)?;
            return Ok(Some(result));
        }

        let result = $fetch_fn.await?;
        if let Some(ref data) = result {
            let json_str = serde_json::to_string(data).map_err(TransportErrorKind::custom)?;
            let _ = $self.put($hash, json_str)?;
        }

        Ok(result)
    }};
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
        let hash = RequestType::BlockByNumber((number, hydrate)).params_hash()?;

        cache_get_or_fetch!(self, hash, self.inner.get_block_by_number(number, hydrate))
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

        let req_hash = RequestType::BlockByHash((hash, full)).params_hash()?;

        cache_get_or_fetch!(self, req_hash, self.inner.get_block_by_hash(hash, kind))
    }

    // TODO: Add other commonly used methods such as eth_getTransactionByHash,
    // eth_getTransactionReceipt, etc.

    /// Get the account and storage values of the specified account including the merkle proofs.
    ///
    /// This call can be used to verify that the data has not been tampered with.
    fn get_proof(
        &self,
        address: Address,
        keys: Vec<StorageKey>,
    ) -> RpcWithBlock<T, (Address, Vec<StorageKey>), EIP1186AccountProofResponse> {
        let client = self.inner.weak_client();
        let cache = self.cache.clone();
        RpcWithBlock::new_provider(move |block_id| {
            let req = RequestType::Proof((address, keys.clone(), block_id));
            let hash = req.params_hash().ok();

            if let Some(hash) = hash {
                // let cache = cache.read();
                if let Some(cached) = cache.write().get(&hash) {
                    let result = serde_json::from_str(cached).map_err(TransportErrorKind::custom);

                    return ProviderCall::BoxedFuture(Box::pin(async move {
                        let res = result?;
                        Ok(res)
                    }));
                }
            }

            let client = client.upgrade().ok_or_else(|| {
                TransportErrorKind::custom_str("RPC client was dropped before the request was made")
            });
            let cache = cache.clone();
            let keys = keys.clone();
            ProviderCall::BoxedFuture(Box::pin(async move {
                let client = client?;

                let result = client
                    .request("eth_getProof", (address, keys))
                    .map_params(|params| ParamsWithBlock { params, block_id })
                    .map_resp(EIP1186AccountProofResponse::from);

                let res = result.await?;

                // Insert into cache.
                let json_str = serde_json::to_string(&res).map_err(TransportErrorKind::custom)?;
                let hash = req.params_hash()?;
                let mut cache = cache.write();
                let _ = cache.put(hash, json_str);

                Ok(res)
            }))
        })
    }

    /// Gets the specified storage value from [Address].
    fn get_storage_at(
        &self,
        _address: Address,
        _key: U256,
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
    BlockByNumber(Params),
    /// Get block by hash.
    BlockByHash(Params),
    /// Get proof.
    Proof(Params),
    /// Get storage at.
    #[allow(dead_code)] // todo
    StorageAt(Params),
}

impl<Params: RpcParam> RequestType<Params> {
    fn params_hash(&self) -> TransportResult<B256> {
        let (_method, params) = match self {
            Self::BlockByNumber(params) => ("eth_getBlockByNumber", params),
            Self::BlockByHash(params) => ("eth_getBlockByHash", params),
            Self::Proof(params) => ("eth_getProof", params),
            Self::StorageAt(params) => ("eth_getStorageAt", params),
        };

        let hash = serde_json::to_string(&params)
            .map(|p| keccak256(p.as_bytes()))
            .map_err(RpcError::ser_err)?;

        Ok(hash)
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
    use alloy_network::TransactionBuilder;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{Bytes, FixedBytes};
    use alloy_rpc_types_eth::{BlockId, TransactionRequest};

    use super::*;

    #[tokio::test]
    async fn test_cache_provider() {
        let cache = CacheLayer::new(100);
        let anvil = Anvil::new().block_time_f64(0.3).spawn();
        let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());

        let path = PathBuf::from_str("./rpc-cache-block-by-number.txt").unwrap();
        provider.load_cache(path.clone()).unwrap();

        let blk = provider.get_block_by_number(0.into(), true).await.unwrap();
        let blk2 = provider.get_block_by_number(0.into(), true).await.unwrap();
        assert_eq!(blk, blk2);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let latest_block_num = provider.get_block_number().await.unwrap();
        let blk3 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();
        let blk4 = provider.get_block_by_number(latest_block_num.into(), true).await.unwrap();
        assert_eq!(blk3, blk4);

        provider.save_cache(path).unwrap();
    }

    #[tokio::test]
    async fn test_get_block() {
        let cache = CacheLayer::new(100);
        let anvil = Anvil::new().block_time_f64(0.3).spawn();
        let provider = ProviderBuilder::default().layer(cache).on_http(anvil.endpoint_url());

        let path = PathBuf::from_str("./rpc-cache-block-by-hash.txt").unwrap();
        provider.load_cache(path.clone()).unwrap();

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

        provider.save_cache(path).unwrap();
    }

    #[tokio::test]
    async fn test_get_proof() {
        let cache = CacheLayer::new(100);
        let anvil = Anvil::new().block_time_f64(0.3).spawn();
        let provider = ProviderBuilder::default()
            // .with_recommended_fillers() - TODO: Issue here. Layer doesn't work with fillers. Fix.
            .layer(cache)
            .on_http(anvil.endpoint_url());

        let from = anvil.addresses()[0];
        let path = PathBuf::from_str("./rpc-cache-proof.txt").unwrap();

        provider.load_cache(path.clone()).unwrap();

        let calldata: Bytes = "0x6080604052348015600f57600080fd5b506101f28061001f6000396000f3fe608060405234801561001057600080fd5b50600436106100415760003560e01c80633fb5c1cb146100465780638381f58a14610062578063d09de08a14610080575b600080fd5b610060600480360381019061005b91906100ee565b61008a565b005b61006a610094565b604051610077919061012a565b60405180910390f35b61008861009a565b005b8060008190555050565b60005481565b6000808154809291906100ac90610174565b9190505550565b600080fd5b6000819050919050565b6100cb816100b8565b81146100d657600080fd5b50565b6000813590506100e8816100c2565b92915050565b600060208284031215610104576101036100b3565b5b6000610112848285016100d9565b91505092915050565b610124816100b8565b82525050565b600060208201905061013f600083018461011b565b92915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b600061017f826100b8565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff82036101b1576101b0610145565b5b60018201905091905056fea264697066735822122067ac0f21f648b0cacd1b7260772852ad4a0f63e2cc174168c51a6887fd5197a964736f6c634300081a0033".parse().unwrap();

        let tx = TransactionRequest::default()
            .with_from(from)
            .with_input(calldata)
            .with_max_fee_per_gas(1_000_000_000)
            .with_max_priority_fee_per_gas(1_000_000)
            .with_gas_limit(1_000_000)
            .with_nonce(0);

        let tx_receipt = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();

        let counter_addr = tx_receipt.contract_address.unwrap();

        let keys = vec![
            FixedBytes::with_last_byte(0),
            FixedBytes::with_last_byte(0x1),
            FixedBytes::with_last_byte(0x2),
            FixedBytes::with_last_byte(0x3),
            FixedBytes::with_last_byte(0x4),
        ];

        let start_t = std::time::Instant::now();
        let _proof =
            provider.get_proof(counter_addr, keys.clone()).block_id(1.into()).await.unwrap();
        let end_t = std::time::Instant::now();

        println!("Time taken: {:?}", end_t - start_t);

        let start_t = std::time::Instant::now();
        let _proof2 = provider.get_proof(counter_addr, keys).block_id(1.into()).await.unwrap();
        let end_t = std::time::Instant::now();

        println!("Time taken: {:?}", end_t - start_t);

        provider.save_cache(path).unwrap();
    }
}
