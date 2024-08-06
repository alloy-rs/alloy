use crate::{TransportError, TransportErrorKind, TransportFut, TransportResult};
use alloy_json_rpc::{
    Id, RequestPacket, Response, ResponsePacket, ResponsePayload, RpcError, SerializedRequest,
};
use alloy_primitives::B256;
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    io::BufReader,
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::trace;
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

        let _loaded = service.load_cache().inspect_err(|e| {
            trace!(?e, "Error loading cache");
        });

        service
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

    /// Resolves a `SerializedRequest` into a `RawValue` if it exists in the cache.
    pub fn resolve(&self, req: &SerializedRequest) -> TransportResult<Option<Box<RawValue>>> {
        let key = req.params_hash();
        let value = self.get(&key)?;

        match value {
            Some(value) => {
                let raw = RawValue::from_string(value).map_err(RpcError::ser_err)?;
                Ok(Some(raw))
            }
            None => Ok(None),
        }
    }

    /// Handles a cache hit.
    fn handle_cache_hit(&self, id: Id, raw: Box<RawValue>) -> ResponsePacket {
        let payload = ResponsePayload::Success(raw);
        let response = Response { id, payload };
        ResponsePacket::Single(response)
    }

    /// Saves the cache to a file specified by the path.
    /// If the files does not exist, it creates one.
    /// If the file exists, it overwrites it.
    pub fn save_cache(&self) -> TransportResult<()> {
        let path = self.config.path.clone();
        let file = std::fs::File::create(path).map_err(TransportErrorKind::custom)?;
        let cache = self.cache.read();

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
        trace!("Loading cache...");
        let path = self.config.path.clone();
        if !path.exists() {
            trace!(?path, "Cache file does not exist.");
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
                match this.resolve(&ser_req) {
                    Ok(Some(raw)) => {
                        let resp = this.handle_cache_hit(ser_req.id().to_owned(), raw);
                        Box::pin(async move { Ok(resp) })
                    }
                    Ok(None) => {
                        Box::pin(async move {
                            match inner.call(req).await {
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
            RequestPacket::Batch(_) => Box::pin(async move {
                // Ignores cache, forwards request.
                match inner.call(req).await {
                    Ok(resp) => Ok(resp),
                    Err(e) => Err(e),
                }
            }),
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
