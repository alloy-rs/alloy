use crate::time::Instant;
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use core::time::Duration;
use derive_more::{Deref, DerefMut};
use futures::{stream::FuturesUnordered, StreamExt};
use parking_lot::RwLock;
use std::{
    collections::{HashSet, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::trace;

use crate::{TransportError, TransportErrorKind, TransportFut};

// Constants for the transport ranking algorithm
const STABILITY_WEIGHT: f64 = 0.7;
const LATENCY_WEIGHT: f64 = 0.3;
const DEFAULT_SAMPLE_COUNT: usize = 10;
const DEFAULT_ACTIVE_TRANSPORT_COUNT: usize = 3;

/// The [`FallbackService`] consumes multiple transports and is able to
/// query them in parallel, returning the first successful response.
///
/// The service ranks transports based on latency and stability metrics,
/// and will attempt to always use the best available transports.
#[derive(Debug, Clone)]
pub struct FallbackService<S> {
    /// The list of transports to use
    transports: Arc<Vec<ScoredTransport<S>>>,
    /// The maximum number of transports to use in parallel
    active_transport_count: usize,
    /// Set of RPC methods that require sequential execution (non-deterministic results in
    /// parallel)
    sequential_methods: Arc<HashSet<String>>,
}

impl<S: Clone> FallbackService<S> {
    /// Create a new fallback service from a list of transports.
    ///
    /// The `active_transport_count` parameter controls how many transports are used for requests
    /// at any one time.
    ///
    /// Uses the default set of sequential methods (eth_sendRawTransactionSync,
    /// eth_sendTransactionSync).
    pub fn new(transports: Vec<S>, active_transport_count: usize) -> Self {
        Self::new_with_sequential_methods(
            transports,
            active_transport_count,
            default_sequential_methods(),
        )
    }

    /// Create a new fallback service from a list of transports.
    ///
    /// The `active_transport_count` parameter controls how many transports are used for requests
    /// at any one time.
    ///
    /// Uses the given set of sequential methods (eth_sendRawTransactionSync,
    /// eth_sendTransactionSync).
    pub fn new_with_sequential_methods(
        transports: Vec<S>,
        active_transport_count: usize,
        sequential_methods: HashSet<String>,
    ) -> Self {
        let scored_transports = transports
            .into_iter()
            .enumerate()
            .map(|(id, transport)| ScoredTransport::new(id, transport))
            .collect::<Vec<_>>();

        Self {
            transports: Arc::new(scored_transports),
            active_transport_count,
            sequential_methods: Arc::new(sequential_methods),
        }
    }

    /// Inserts the sequential method into the set.
    pub fn append_sequential_method(mut self, sequential_method: impl Into<String>) -> Self {
        let mut methods = Arc::unwrap_or_clone(self.sequential_methods);
        methods.insert(sequential_method.into());
        self.sequential_methods = Arc::new(methods);
        self
    }

    /// Configures the `sequential_methods` parameter specifies which RPC methods require sequential
    /// execution due to non-deterministic results in parallel execution.
    pub fn with_sequential_methods(mut self, sequential_methods: HashSet<String>) -> Self {
        self.sequential_methods = Arc::new(sequential_methods);
        self
    }

    /// Log the current ranking of transports
    fn log_transport_rankings(&self) {
        if !tracing::enabled!(tracing::Level::TRACE) {
            return;
        }

        // Prepare lightweight ranking data without cloning transports
        let mut ranked: Vec<(usize, f64, String)> =
            self.transports.iter().map(|t| (t.id, t.score(), t.metrics_summary())).collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        trace!("Current transport rankings:");
        for (idx, (id, _score, summary)) in ranked.iter().enumerate() {
            trace!("  #{}: Transport[{}] - {}", idx + 1, id, summary);
        }
    }

    /// Returns the top transports sorted by score (best first), limited by
    /// `active_transport_count`.
    fn top_transports(&self) -> Vec<ScoredTransport<S>> {
        // Clone the vec, sort it, and keep only the top `self.active_transport_count`.
        let mut transports_clone = (*self.transports).clone();
        transports_clone.sort_by(|a, b| b.cmp(a));
        transports_clone.truncate(self.active_transport_count);
        transports_clone
    }
}

impl<S> FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static,
{
    /// Make a request to the fallback service middleware.
    ///
    /// Here is a high-level overview of how requests are handled:
    ///
    /// **For methods with non-deterministic results** (e.g., `eth_sendRawTransactionSync`):
    /// - Methods are tried sequentially on each transport
    /// - Returns the first successful response
    /// - Prevents returning wrong results (e.g., "already known" instead of receipt)
    ///
    /// **For methods with deterministic results** (default - most methods):
    /// - At the start of each request, we sort transports by score
    /// - We take the top `self.active_transport_count` and call them in parallel
    /// - If any of them succeeds, we update the transport scores and return the response
    /// - If all transports fail, we update the scores and return the last error that occurred
    ///
    /// This strategy allows us to always make requests to the best available transports
    /// while ensuring correctness for methods that return different results in parallel.
    async fn make_request(&self, req: RequestPacket) -> Result<ResponsePacket, TransportError> {
        // Check if any method in the request requires sequential execution
        // For batch requests: if ANY method needs sequential execution, the entire batch must be
        // sequential
        if req.method_names().any(|name| self.sequential_methods.contains(name)) {
            return self.make_request_sequential(req).await;
        }

        // Default: parallel execution for methods with deterministic results
        // Get the top transports to use for this request
        let top_transports = self.top_transports();

        // Create a collection of future requests
        let mut futures = FuturesUnordered::new();

        // Launch requests to all active transports in parallel
        for mut transport in top_transports {
            let req_clone = req.clone();

            let future = async move {
                let start = Instant::now();
                let result = transport.call(req_clone).await;
                trace!(
                    "Transport[{}] completed: latency={:?}, status={}",
                    transport.id,
                    start.elapsed(),
                    if result.is_ok() { "success" } else { "fail" }
                );

                (result, transport, start.elapsed())
            };

            futures.push(future);
        }

        // Wait for the first successful response or until all fail
        let mut last_error = None;

        while let Some((result, transport, duration)) = futures.next().await {
            match result {
                Ok(response) => {
                    // Record success
                    transport.track_success(duration);

                    self.log_transport_rankings();

                    return Ok(response);
                }
                Err(error) => {
                    // Record failure
                    transport.track_failure();

                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            TransportErrorKind::custom_str("All transport futures failed to complete")
        }))
    }

    /// Make a sequential request for methods with non-deterministic results.
    ///
    /// This method tries each transport one at a time, in order of their score.
    /// It returns the first successful response, or an error if all transports fail.
    ///
    /// This approach ensures methods like `eth_sendRawTransactionSync` return the correct
    /// receipt instead of "already known" errors from parallel execution.
    async fn make_request_sequential(
        &self,
        req: RequestPacket,
    ) -> Result<ResponsePacket, TransportError> {
        trace!("Using sequential fallback for method with non-deterministic results");

        // Get transports sorted by score (best first)
        let top_transports = self.top_transports();

        let mut last_error = None;

        // Try each transport sequentially
        for mut transport in top_transports {
            let req_clone = req.clone();
            let start = Instant::now();

            trace!("Trying transport[{}] sequentially", transport.id);

            match transport.call(req_clone).await {
                Ok(response) => {
                    // Record success and return immediately
                    transport.track_success(start.elapsed());
                    trace!("Transport[{}] succeeded in {:?}", transport.id, start.elapsed());
                    self.log_transport_rankings();
                    return Ok(response);
                }
                Err(error) => {
                    // Record failure and try next transport
                    transport.track_failure();
                    trace!("Transport[{}] failed: {:?}, trying next", transport.id, error);
                    last_error = Some(error);
                }
            }
        }

        // All transports failed
        Err(last_error.unwrap_or_else(|| {
            TransportErrorKind::custom_str("All transports failed for sequential request")
        }))
    }
}

impl<S> Service<RequestPacket> for FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Sync
        + Clone
        + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Service is always ready
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.make_request(req).await })
    }
}

/// Fallback layer for transparent transport failover. This layer will
/// consume a list of transports to provide better availability and
/// reliability.
///
/// The [`FallbackService`] will attempt to make requests to multiple
/// transports in parallel, and return the first successful response.
///
/// If all transports fail, the fallback service will return an error.
///
/// # Automatic Transport Ranking
///
/// Each transport is automatically ranked based on latency & stability
/// using a weighted algorithm. By default:
///
/// - Stability (success rate) is weighted at 70%
/// - Latency (response time) is weighted at 30%
/// - The `active_transport_count` parameter controls how many transports are queried at any one
///   time.
#[derive(Debug, Clone)]
pub struct FallbackLayer {
    /// The maximum number of transports to use in parallel
    active_transport_count: usize,
    /// Set of RPC methods that require sequential execution (non-deterministic results in
    /// parallel)
    sequential_methods: HashSet<String>,
}

impl FallbackLayer {
    /// Set the number of active transports to use (must be greater than 0)
    pub const fn with_active_transport_count(mut self, count: NonZeroUsize) -> Self {
        self.active_transport_count = count.get();
        self
    }

    /// Add an RPC method that requires sequential execution.
    ///
    /// Sequential execution is needed for methods that return non-deterministic results
    /// when executed in parallel across multiple nodes (e.g., methods that wait for confirmations).
    pub fn with_sequential_method(mut self, method: impl Into<String>) -> Self {
        self.sequential_methods.insert(method.into());
        self
    }

    /// Set the complete list of RPC methods that require sequential execution.
    ///
    /// This replaces the default set. Use this if you want full control over which methods
    /// use sequential execution.
    pub fn with_sequential_methods(mut self, methods: HashSet<String>) -> Self {
        self.sequential_methods = methods;
        self
    }

    /// Clear all sequential methods (all requests will use parallel execution).
    ///
    /// **Warning**: Only use this if you're certain none of your RPC methods have
    /// non-deterministic results in parallel execution.
    pub fn without_sequential_methods(mut self) -> Self {
        self.sequential_methods.clear();
        self
    }
}

impl<S> Layer<Vec<S>> for FallbackLayer
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static,
{
    type Service = FallbackService<S>;

    fn layer(&self, inner: Vec<S>) -> Self::Service {
        FallbackService::new_with_sequential_methods(
            inner,
            self.active_transport_count,
            self.sequential_methods.clone(),
        )
    }
}

impl Default for FallbackLayer {
    fn default() -> Self {
        Self {
            active_transport_count: DEFAULT_ACTIVE_TRANSPORT_COUNT,
            sequential_methods: default_sequential_methods(),
        }
    }
}

/// A scored transport that can be ordered in a heap.
///
/// The transport is scored every time it is used according to
/// a simple weighted algorithm that favors latency and stability.
///
/// The score is calculated as follows (by default):
///
/// - Stability (success rate) is weighted at 70%
/// - Latency (response time) is weighted at 30%
///
/// The score is then used to determine which transport to use next in
/// the [`FallbackService`].
#[derive(Debug, Clone, Deref, DerefMut)]
struct ScoredTransport<S> {
    /// The transport itself
    #[deref]
    #[deref_mut]
    transport: S,
    /// Unique identifier for the transport
    id: usize,
    /// Metrics for the transport
    metrics: Arc<RwLock<TransportMetrics>>,
}

impl<S> ScoredTransport<S> {
    /// Create a new scored transport
    fn new(id: usize, transport: S) -> Self {
        Self { id, transport, metrics: Arc::new(Default::default()) }
    }

    /// Returns the current score of the transport based on the weighted algorithm.
    fn score(&self) -> f64 {
        let metrics = self.metrics.read();
        metrics.calculate_score()
    }

    /// Get metrics summary for debugging
    fn metrics_summary(&self) -> String {
        let metrics = self.metrics.read();
        metrics.get_summary()
    }

    /// Track a successful request and its latency.
    fn track_success(&self, duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.track_success(duration);
    }

    /// Track a failed request.
    fn track_failure(&self) {
        let mut metrics = self.metrics.write();
        metrics.track_failure();
    }
}

impl<S> PartialEq for ScoredTransport<S> {
    fn eq(&self, other: &Self) -> bool {
        self.score().eq(&other.score())
    }
}

impl<S> Eq for ScoredTransport<S> {}

#[expect(clippy::non_canonical_partial_ord_impl)]
impl<S> PartialOrd for ScoredTransport<S> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score().partial_cmp(&other.score())
    }
}

impl<S> Ord for ScoredTransport<S> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Represents performance metrics for a transport.
#[derive(Debug)]
struct TransportMetrics {
    // Latency history - tracks last N responses
    latencies: VecDeque<Duration>,
    // Success history - tracks last N successes (true) or failures (false)
    successes: VecDeque<bool>,
    // Last time this transport was checked/used
    last_update: Instant,
    // Total number of requests made to this transport
    total_requests: u64,
    // Total number of successful requests
    successful_requests: u64,
}

impl TransportMetrics {
    /// Track a successful request and its latency.
    fn track_success(&mut self, duration: Duration) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_update = Instant::now();

        // Add to sample windows
        self.latencies.push_back(duration);
        self.successes.push_back(true);

        // Limit to sample count
        while self.latencies.len() > DEFAULT_SAMPLE_COUNT {
            self.latencies.pop_front();
        }
        while self.successes.len() > DEFAULT_SAMPLE_COUNT {
            self.successes.pop_front();
        }
    }

    /// Track a failed request.
    fn track_failure(&mut self) {
        self.total_requests += 1;
        self.last_update = Instant::now();

        // Add to sample windows (no latency for failures)
        self.successes.push_back(false);

        // Limit to sample count
        while self.successes.len() > DEFAULT_SAMPLE_COUNT {
            self.successes.pop_front();
        }
    }

    /// Calculate weighted score based on stability and latency
    fn calculate_score(&self) -> f64 {
        // If no data yet, return initial neutral score
        if self.successes.is_empty() {
            return 0.0;
        }

        // Calculate stability score (percentage of successful requests)
        let success_count = self.successes.iter().filter(|&&s| s).count();
        let stability_score = success_count as f64 / self.successes.len() as f64;

        // Calculate latency score (lower is better)
        let latency_score = if !self.latencies.is_empty() {
            let avg_latency = self.latencies.iter().map(|d| d.as_secs_f64()).sum::<f64>()
                / self.latencies.len() as f64;

            // Normalize latency score (1.0 for 0ms, approaches 0.0 as latency increases)
            1.0 / (1.0 + avg_latency)
        } else {
            0.0
        };

        // Apply weights to calculate final score
        (stability_score * STABILITY_WEIGHT) + (latency_score * LATENCY_WEIGHT)
    }

    /// Get a summary of metrics for debugging
    fn get_summary(&self) -> String {
        let success_rate = if !self.successes.is_empty() {
            let success_count = self.successes.iter().filter(|&&s| s).count();
            success_count as f64 / self.successes.len() as f64
        } else {
            0.0
        };

        let avg_latency = if !self.latencies.is_empty() {
            self.latencies.iter().map(|d| d.as_secs_f64()).sum::<f64>()
                / self.latencies.len() as f64
        } else {
            0.0
        };

        format!(
            "success_rate: {:.2}%, avg_latency: {:.2}ms, samples: {}, score: {:.4}",
            success_rate * 100.0,
            avg_latency * 1000.0,
            self.successes.len(),
            self.calculate_score()
        )
    }
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self {
            latencies: VecDeque::new(),
            successes: VecDeque::new(),
            last_update: Instant::now(),
            total_requests: 0,
            successful_requests: 0,
        }
    }
}

/// Returns the default set of RPC methods that require sequential execution.
///
/// These methods return different valid results when the same request is sent to multiple
/// nodes in parallel, requiring sequential execution to ensure correct results.
///
/// Methods in this list share a common pattern:
/// - They wait for transaction confirmation before returning
/// - First node: submits tx → waits → returns receipt
/// - Other nodes: tx already in mempool → return "already known" error
/// - Result: parallel execution returns error instead of receipt
///
/// Sequential execution tries transports one at a time, in order of their score.
/// Only moves to the next transport if the previous one fails. This ensures we
/// always get the correct result while maintaining fallback capability.
///
/// # Default Methods:
/// - `eth_sendRawTransactionSync` (EIP-7966): waits for receipt
/// - `eth_sendTransactionSync`: same as above but for unsigned transactions
fn default_sequential_methods() -> HashSet<String> {
    ["eth_sendRawTransactionSync".to_string(), "eth_sendTransactionSync".to_string()]
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request, Response, ResponsePayload};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{sleep, Duration};
    use tower::Service;

    /// A mock transport that can be configured to return responses with delays
    #[derive(Clone)]
    struct DelayedMockTransport {
        delay: Duration,
        response: Arc<RwLock<Option<ResponsePayload>>>,
        call_count: Arc<AtomicUsize>,
    }

    impl DelayedMockTransport {
        fn new(delay: Duration, response: ResponsePayload) -> Self {
            Self {
                delay,
                response: Arc::new(RwLock::new(Some(response))),
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl Service<RequestPacket> for DelayedMockTransport {
        type Response = ResponsePacket;
        type Error = TransportError;
        type Future = TransportFut<'static>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: RequestPacket) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let delay = self.delay;
            let response = self.response.clone();

            Box::pin(async move {
                sleep(delay).await;

                match req {
                    RequestPacket::Single(single) => {
                        let resp = response.read().clone().ok_or_else(|| {
                            TransportErrorKind::custom_str("No response configured")
                        })?;

                        Ok(ResponsePacket::Single(Response {
                            id: single.id().clone(),
                            payload: resp,
                        }))
                    }
                    RequestPacket::Batch(batch) => {
                        let resp = response.read().clone().ok_or_else(|| {
                            TransportErrorKind::custom_str("No response configured")
                        })?;

                        // Return the same response for each request in the batch
                        let responses = batch
                            .iter()
                            .map(|req| Response { id: req.id().clone(), payload: resp.clone() })
                            .collect();

                        Ok(ResponsePacket::Batch(responses))
                    }
                }
            })
        }
    }

    /// Helper to create a successful response with given data
    fn success_response(data: &str) -> ResponsePayload {
        let raw = serde_json::value::RawValue::from_string(format!("\"{}\"", data)).unwrap();
        ResponsePayload::Success(raw)
    }

    #[tokio::test]
    async fn test_non_deterministic_method_uses_sequential_fallback() {
        // Test that eth_sendRawTransactionSync (which returns non-deterministic results
        // in parallel) uses sequential fallback and returns the correct receipt, not "already
        // known"

        let transport_a = DelayedMockTransport::new(
            Duration::from_millis(50),
            success_response("0x1234567890abcdef"), // Actual receipt
        );

        let transport_b = DelayedMockTransport::new(
            Duration::from_millis(10),
            success_response("already_known"), // Fast but wrong
        );

        let transports = vec![transport_a.clone(), transport_b.clone()];
        let mut fallback_service = FallbackService::new(transports, 2);

        let request = Request::new(
            "eth_sendRawTransactionSync",
            Id::Number(1),
            [serde_json::Value::String("0xabcdef".to_string())],
        );
        let serialized = request.serialize().unwrap();
        let request_packet = RequestPacket::Single(serialized);

        let start = std::time::Instant::now();
        let response = fallback_service.call(request_packet).await.unwrap();
        let elapsed = start.elapsed();

        let result = match response {
            ResponsePacket::Single(resp) => match resp.payload {
                ResponsePayload::Success(data) => data.get().to_string(),
                ResponsePayload::Failure(err) => panic!("Unexpected error: {:?}", err),
            },
            ResponsePacket::Batch(_) => panic!("Unexpected batch response"),
        };

        // Should only call the first transport sequentially (succeeds immediately)
        assert_eq!(transport_a.call_count(), 1, "First transport should be called");
        // Should NOT call second transport since first succeeded
        assert_eq!(transport_b.call_count(), 0, "Second transport should NOT be called");

        // Should return the actual receipt, not "already_known"
        assert_eq!(result, "\"0x1234567890abcdef\"");

        // Should take ~50ms (first transport only), not ~10ms (second transport)
        assert!(
            elapsed >= Duration::from_millis(40),
            "Should wait for first transport: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_deterministic_method_uses_parallel_execution() {
        // Test that eth_sendRawTransaction (which returns deterministic results)
        // uses parallel execution because the tx hash is the same from all nodes

        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let transport_a = DelayedMockTransport::new(
            Duration::from_millis(100),
            success_response(tx_hash), // Same hash
        );

        let transport_b = DelayedMockTransport::new(
            Duration::from_millis(20),
            success_response(tx_hash), // Same hash, faster
        );

        let transports = vec![transport_a.clone(), transport_b.clone()];
        let mut fallback_service = FallbackService::new(transports, 2);

        let request = Request::new(
            "eth_sendRawTransaction",
            Id::Number(1),
            [serde_json::Value::String("0xabcdef".to_string())],
        );
        let serialized = request.serialize().unwrap();
        let request_packet = RequestPacket::Single(serialized);

        let start = std::time::Instant::now();
        let response = fallback_service.call(request_packet).await.unwrap();
        let elapsed = start.elapsed();

        let result = match response {
            ResponsePacket::Single(resp) => match resp.payload {
                ResponsePayload::Success(data) => data.get().to_string(),
                ResponsePayload::Failure(err) => panic!("Unexpected error: {:?}", err),
            },
            ResponsePacket::Batch(_) => panic!("Unexpected batch response"),
        };

        // Both transports should be called in parallel
        assert_eq!(transport_a.call_count(), 1, "Transport A should be called");
        assert_eq!(transport_b.call_count(), 1, "Transport B should be called");

        // Should return the tx hash (same from both)
        assert_eq!(result, format!("\"{}\"", tx_hash));

        // Should complete in ~20ms (fast transport), not ~100ms (slow transport)
        assert!(
            elapsed < Duration::from_millis(50),
            "Should use parallel execution and return fast: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_batch_with_any_sequential_method_uses_sequential_execution() {
        // Test that if ANY method in a batch requires sequential execution,
        // the entire batch is executed sequentially

        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        // Transport A: Fast, returns success for both methods
        let transport_a =
            DelayedMockTransport::new(Duration::from_millis(10), success_response(tx_hash));

        // Transport B: Also fast, but would return error (but shouldn't be called in sequential
        // mode)
        let transport_b = DelayedMockTransport::new(
            Duration::from_millis(10),
            success_response("should_not_be_called"),
        );

        let transports = vec![transport_a.clone(), transport_b.clone()];
        let mut fallback_service = FallbackService::new(transports, 2);

        // Create a batch with:
        // 1. eth_blockNumber (deterministic, normally parallel)
        // 2. eth_sendRawTransactionSync (non-deterministic, requires sequential)
        let request1 = Request::new("eth_blockNumber", Id::Number(1), ());
        let request2 = Request::new(
            "eth_sendRawTransactionSync",
            Id::Number(2),
            [serde_json::Value::String("0xabcdef".to_string())],
        );

        let batch = vec![request1.serialize().unwrap(), request2.serialize().unwrap()];
        let request_packet = RequestPacket::Batch(batch);

        let start = std::time::Instant::now();
        let response = fallback_service.call(request_packet).await.unwrap();
        let elapsed = start.elapsed();

        // In sequential mode: only transport_a should be called (it succeeds)
        // transport_b should NOT be called because transport_a already succeeded
        assert_eq!(
            transport_a.call_count(),
            1,
            "Transport A should be called once (first in sequence)"
        );
        assert_eq!(
            transport_b.call_count(),
            0,
            "Transport B should NOT be called (transport A succeeded)"
        );

        // Verify we got the correct response
        match response {
            ResponsePacket::Batch(responses) => {
                assert_eq!(responses.len(), 2, "Should get 2 responses in batch");
                // Both should be successful responses from transport A
                for resp in responses {
                    match resp.payload {
                        ResponsePayload::Success(_) => {} // Expected
                        ResponsePayload::Failure(err) => panic!("Unexpected error: {:?}", err),
                    }
                }
            }
            ResponsePacket::Single(_) => panic!("Expected batch response"),
        }

        // Should complete quickly since transport A is fast (10ms)
        assert!(
            elapsed < Duration::from_millis(50),
            "Sequential execution with fast first transport should be quick: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_custom_sequential_method() {
        // Test that users can add custom methods to the sequential execution list

        // Transport A: Fast, always succeeds
        let transport_a =
            DelayedMockTransport::new(Duration::from_millis(10), success_response("result_a"));

        // Transport B: Also fast, returns different result
        let transport_b =
            DelayedMockTransport::new(Duration::from_millis(10), success_response("result_b"));

        let transports = vec![transport_a.clone(), transport_b.clone()];

        // Create FallbackService with custom sequential method "my_custom_method"
        let custom_methods = ["my_custom_method".to_string()].into_iter().collect();
        let mut fallback_service =
            FallbackService::new(transports, 2).with_sequential_methods(custom_methods);

        let request = Request::new("my_custom_method", Id::Number(1), ());
        let serialized = request.serialize().unwrap();
        let request_packet = RequestPacket::Single(serialized);

        let start = std::time::Instant::now();
        let _response = fallback_service.call(request_packet).await.unwrap();
        let elapsed = start.elapsed();

        // Should use sequential execution:
        // - Only transport_a called (first in list, succeeds)
        // - transport_b NOT called (sequential mode stops after first success)
        assert_eq!(
            transport_a.call_count(),
            1,
            "Transport A should be called once (sequential, first transport)"
        );
        assert_eq!(
            transport_b.call_count(),
            0,
            "Transport B should NOT be called (sequential mode, A succeeded)"
        );

        // Should complete in ~10ms (only transport A called)
        assert!(
            elapsed < Duration::from_millis(50),
            "Sequential execution with fast first transport: {:?}",
            elapsed
        );
    }
}
