use crate::{TransportError, TransportErrorKind, TransportFut, TransportResult};
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload};
use alloy_primitives::B256;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    io::BufReader,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tower::{Layer, Service};
/// Cache Layer
#[derive(Debug, Clone)]
pub struct CacheLayer {
    /// Config for the cache layer.
    config: CacheConfig,
}

impl CacheLayer {
    /// Instantiate a new cache layer with the the maximum number of
    /// items to store.
    #[inline]
    pub const fn new(max_items: usize, path: PathBuf) -> Self {
        Self { config: CacheConfig { max_items, path } }
    }

    /// Returns the maximum number of items that can be stored in the cache, set at initialization.
    #[inline]
    pub const fn max_items(&self) -> usize {
        self.config.max_items
    }
}

/// Configuration for the cache layer.
/// For future extensibility of the configurations.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of items to store in the cache.
    pub max_items: usize,
    /// Path of the cache file.
    pub path: PathBuf,
}

impl<S> Layer<S> for CacheLayer {
    type Service = CachingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CachingService::new(inner, self.config.clone())
    }
}

/// Caching service.
#[derive(Debug, Clone)]
pub struct CachingService<S> {
    /// Inner transport service.
    inner: S,
    /// Config for the cache layer.
    config: CacheConfig,
    /// In-memory LRU cache, mapping requests to responses.
    cache: Arc<RwLock<LruCache<B256, String>>>,
}

impl<S> Drop for CachingService<S> {
    fn drop(&mut self) {
        let _ = self.save_cache();
    }
}

impl<S> CachingService<S> {
    /// Instantiate a new cache service.
    pub fn new(inner: S, config: CacheConfig) -> Self {
        let cache = Arc::new(RwLock::new(LruCache::<B256, String>::new(
            NonZeroUsize::new(config.max_items).unwrap(),
        )));
        let service = Self { inner, config, cache };

        let loaded = service.load_cache();

        match loaded {
            Ok(_) => {
                tracing::info!("Loaded cache");
            }
            Err(e) => {
                tracing::info!("Error loading cache: {:?}", e);
            }
        }
        service
    }

    /// Puts a value into the cache, and returns the old value if it existed.
    pub fn put(&self, key: B256, value: String) -> TransportResult<Option<String>> {
        let mut cache = self.cache.write().unwrap();
        Ok(cache.put(key, value))
    }

    /// Gets a value from the cache, if it exists.
    pub fn get(&self, key: &B256) -> TransportResult<Option<String>> {
        // Need to acquire a write guard to change the order of keys in LRU cache.
        let mut cache = self.cache.write().unwrap();
        let val = cache.get(key).cloned();
        Ok(val)
    }

    /// Saves the cache to a file specified by the path.
    /// If the files does not exist, it creates one.
    /// If the file exists, it overwrites it.
    pub fn save_cache(&self) -> TransportResult<()> {
        let path = self.config.path.clone();
        let file = std::fs::File::create(path).map_err(TransportErrorKind::custom)?;
        let cache = self.cache.read().unwrap();

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
    pub fn load_cache(&self) -> TransportResult<()> {
        println!("Loading cache...");
        let path = self.config.path.clone();
        if !path.exists() {
            println!("Cache file does not exist.");
            return Ok(());
        };
        let file = std::fs::File::open(path).map_err(TransportErrorKind::custom)?;
        let file = BufReader::new(file);
        let entries: Vec<FsCacheEntry> =
            serde_json::from_reader(file).map_err(TransportErrorKind::custom)?;
        let mut cache = self.cache.write().unwrap();
        for entry in entries {
            cache.put(entry.key, entry.value);
        }

        println!("Loaded from Cache");
        Ok(())
    }
}

impl<S> Service<RequestPacket> for CachingService<S>
where
    S: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>
        + Send
        + 'static
        + Clone,
    S::Future: Send + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), TransportError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let mut inner = self.inner.clone();
        let this = self.clone();
        match req.clone() {
            RequestPacket::Single(ser_req) => {
                let params_hash = ser_req.params_hash();
                let resp = this.get(&params_hash);
                match resp {
                    Ok(Some(resp)) => {
                        println!("Cache hit!");
                        let raw = RawValue::from_string(resp).unwrap();
                        let payload: ResponsePayload<Box<RawValue>, Box<RawValue>> =
                            ResponsePayload::Success(raw);
                        let response = Response { id: ser_req.id().clone(), payload };

                        Box::pin(async move { Ok(ResponsePacket::Single(response)) })
                    }
                    Ok(None) => {
                        println!("Cache miss!");
                        Box::pin(async move {
                            let res = inner.call(req).await;
                            match res {
                                Ok(resp) => {
                                    // Store success response into cache.
                                    if let Some(res) = resp.single_response() {
                                        let ser = res.payload.as_success().unwrap().to_string();
                                        let _ = this.put(params_hash, ser);
                                    }

                                    Ok(resp)
                                }
                                Err(e) => Err(e),
                            }
                        })
                    }
                    Err(e) => Box::pin(async move { Err(e) }),
                }
            }
            RequestPacket::Batch(_reqs) => {
                todo!()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FsCacheEntry {
    /// Hash of the request params
    key: B256,
    /// Serialized response to the request from which the hash was computed.
    value: String,
}
