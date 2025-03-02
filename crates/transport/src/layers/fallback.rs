use std::{
    collections::{BinaryHeap, VecDeque},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use alloy_json_rpc::{RequestPacket, ResponsePacket};
use futures::{stream::FuturesUnordered, StreamExt};
use tower::{Layer, Service};
use tracing::{debug, trace};

use crate::{TransportError, TransportErrorKind, TransportFut};

// Constants for the ranking algorithm
const STABILITY_WEIGHT: f64 = 0.7;
const LATENCY_WEIGHT: f64 = 0.3;
const DEFAULT_SAMPLE_COUNT: usize = 10;
const DEFAULT_ACTIVE_TRANSPORT_COUNT: usize = 3;

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

/// A scored transport that can be ordered in a heap.
#[derive(Debug, Clone)]
struct ScoredTransport<S> {
    id: usize, // Unique identifier for the transport
    transport: S,
    metrics: Arc<Mutex<TransportMetrics>>,
}

impl<S> ScoredTransport<S> {
    fn new(id: usize, transport: S) -> Self {
        Self { id, transport, metrics: Arc::new(Default::default()) }
    }

    /// Returns the current score of the transport based on the weighted algorithm.
    fn score(&self) -> f64 {
        let metrics = self.metrics.lock().expect("Lock poisoned");
        metrics.calculate_score()
    }

    /// Get metrics summary for debugging
    fn metrics_summary(&self) -> String {
        let metrics = self.metrics.lock().expect("Lock poisoned");
        metrics.get_summary()
    }
}

impl<S> Deref for ScoredTransport<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.transport
    }
}

impl<S> DerefMut for ScoredTransport<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transport
    }
}

impl<S> PartialEq for ScoredTransport<S> {
    fn eq(&self, other: &Self) -> bool {
        self.score().eq(&other.score())
    }
}

impl<S> Eq for ScoredTransport<S> {}

#[allow(clippy::non_canonical_partial_ord_impl)]
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

/// The fallback provider consumes multiple transports and uses a
/// ranking strategy to provide responses optimizing for the best
/// latency and availability.
#[derive(Debug, Clone)]
pub struct FallbackService<S> {
    transports: Arc<Mutex<BinaryHeap<ScoredTransport<S>>>>,
    active_transport_count: usize,
    log_transport_rankings: bool,
}

impl<S> FallbackService<S> {
    /// Create a new fallback provider from a list of transports.
    ///
    /// - The `active_transport_count` parameter controls how many transports are used for requests
    ///   at any one time.
    /// - The `log_transport_rankings` parameter controls whether the current transport rankings are
    ///   output as traces. In order to see them, make sure to enable the `alloy_transport` target
    ///   (i.e. `RUST_LOG=alloy_transport=debug`).
    pub fn new(
        transports: Vec<S>,
        active_transport_count: usize,
        log_transport_rankings: bool,
    ) -> Self {
        let scored_transports = transports
            .into_iter()
            .enumerate()
            .map(|(id, transport)| ScoredTransport::new(id, transport))
            .collect::<Vec<_>>();

        Self {
            transports: Arc::new(Mutex::new(BinaryHeap::from(scored_transports))),
            active_transport_count,
            log_transport_rankings,
        }
    }

    /// Get the number of transports
    pub fn transport_count(&self) -> usize {
        let transports = self.transports.lock().expect("Lock poisoned");
        transports.len()
    }

    /// Log the current ranking of transports
    pub fn log_transport_rankings(&self)
    where
        S: std::fmt::Debug,
    {
        let transports = self.transports.lock().expect("Lock poisoned");
        let mut sorted_transports: Vec<_> = transports.iter().collect();
        sorted_transports.sort_by(|a, b| b.cmp(a)); // Sort by score (descending)

        debug!("Current transport rankings:");
        for (idx, transport) in sorted_transports.iter().enumerate() {
            debug!("  #{}: Transport[{}] - {}", idx + 1, transport.id, transport.metrics_summary());
        }
    }
}

impl<S> FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + std::fmt::Debug
        + 'static,
{
    async fn make_request(&self, req: RequestPacket) -> Result<ResponsePacket, TransportError> {
        let mut top_transports = Vec::new();
        {
            let mut transports = self.transports.lock().expect("Lock poisoned");
            for _ in 0..self.active_transport_count.min(transports.len()) {
                if let Some(transport) = transports.pop() {
                    top_transports.push(transport);
                }
            }
        }

        if top_transports.is_empty() {
            return Err(TransportErrorKind::custom_str("No transports available"));
        }

        // Create a collection of future requests
        let mut futures = FuturesUnordered::new();

        // Launch requests to all active transports in parallel
        for transport in top_transports.iter_mut() {
            let start = Instant::now();
            let req_clone = req.clone();
            let mut transport_clone = transport.clone();

            let future = async move {
                let result = transport_clone.call(req_clone).await;
                trace!(
                    "Transport[{}] completed: latency={:?}, status={}",
                    transport_clone.id,
                    start.elapsed(),
                    if result.is_ok() { "success" } else { "fail" }
                );

                (result, transport_clone, start.elapsed())
            };

            futures.push(future);
        }

        // Wait for the first successful response or until all fail
        let mut last_error = None;

        while let Some((result, transport, duration)) = futures.next().await {
            match result {
                Ok(response) => {
                    trace!("Transport[{}] delivered the first response", transport.id);

                    // Record success
                    {
                        let mut metrics = transport.metrics.lock().expect("Lock poisoned");
                        metrics.track_success(duration);
                    }

                    // Put all transports back in the heap
                    {
                        let mut transports = self.transports.lock().expect("Lock poisoned");
                        for transport in top_transports {
                            transports.push(transport);
                        }
                    }

                    // Log current rankings if enabled
                    if self.log_transport_rankings {
                        self.log_transport_rankings();
                    }

                    // clear the remaining futures to avoid unnecessary work
                    futures.clear();

                    return Ok(response);
                }
                Err(error) => {
                    // Record failure
                    {
                        let mut metrics = transport.metrics.lock().expect("Lock poisoned");
                        metrics.track_failure();
                    }

                    last_error = Some(error);
                }
            }
        }

        // If we got here, all transports failed. Put them back in the heap
        {
            let mut transports = self.transports.lock().expect("Lock poisoned");
            for transport in top_transports {
                transports.push(transport);
            }
        }

        Err(last_error.unwrap_or_else(|| TransportErrorKind::custom_str("All transports failed")))
    }
}

impl<S> Service<RequestPacket> for FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + std::fmt::Debug
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
/// - The `active_transport_count` parameter controls how many
///   transports are queried at any one time.
///
/// The `log_transport_rankings` parameter controls whether the
/// current transport rankings are output as traces. In order to
/// see them, make sure to enable the `alloy_transport` target
/// (i.e. `RUST_LOG=alloy_transport=debug`).
#[derive(Debug, Clone)]
pub struct FallbackLayer {
    active_transport_count: usize,
    log_transport_rankings: bool,
}

impl FallbackLayer {
    /// Set the number of active transports to use (must be greater than 0)
    pub fn with_active_transport_count(mut self, count: NonZeroUsize) -> Self {
        self.active_transport_count = count.get();
        self
    }

    /// Set whether to log transport rankings on every request
    /// (most useful for debugging)
    pub fn with_log_transport_rankings(mut self, enabled: bool) -> Self {
        self.log_transport_rankings = enabled;
        self
    }
}

impl Default for FallbackLayer {
    fn default() -> Self {
        Self {
            active_transport_count: DEFAULT_ACTIVE_TRANSPORT_COUNT,
            log_transport_rankings: false,
        }
    }
}

impl<S> Layer<Vec<S>> for FallbackLayer
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static
        + std::fmt::Debug,
{
    type Service = FallbackService<S>;

    fn layer(&self, inner: Vec<S>) -> Self::Service {
        FallbackService::new(inner, self.active_transport_count, self.log_transport_rankings)
    }
}
