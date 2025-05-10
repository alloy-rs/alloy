use std::{
    collections::VecDeque,
    fmt::Debug,
    num::NonZeroUsize,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use alloy_json_rpc::{RequestPacket, ResponsePacket};
use derive_more::{Deref, DerefMut};
use futures::{stream::FuturesUnordered, StreamExt};
use parking_lot::RwLock;
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
}

impl<S: Clone> FallbackService<S> {
    /// Create a new fallback service from a list of transports.
    ///
    /// The `active_transport_count` parameter controls how many transports are used for requests
    /// at any one time.
    pub fn new(transports: Vec<S>, active_transport_count: usize) -> Self {
        let scored_transports = transports
            .into_iter()
            .enumerate()
            .map(|(id, transport)| ScoredTransport::new(id, transport))
            .collect::<Vec<_>>();

        Self { transports: Arc::new(scored_transports), active_transport_count }
    }

    /// Log the current ranking of transports
    fn log_transport_rankings(&self)
    where
        S: Debug,
    {
        let mut transports = (*self.transports).clone();
        transports.sort_by(|a, b| b.cmp(a));

        trace!(
            target: "alloy_fallback_transport_rankings",
            "Current transport rankings:"
        );
        for (idx, transport) in transports.iter().enumerate() {
            trace!(
                target: "alloy_fallback_transport_rankings",
                "  #{}: Transport[{}] - {}", idx + 1, transport.id, transport.metrics_summary()
            );
        }
    }
}

impl<S> FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + Debug
        + 'static,
{
    /// Make a request to the fallback service middleware.
    ///
    /// Here is a high-level overview of how requests are handled:
    ///
    /// - At the start of each request, we sort transports by score
    /// - We take the top `self.active_transport_count` and call them in parallel
    /// - If any of them succeeds, we update the transport scores and return the response
    /// - If all transports fail, we update the scores and return the last error that occurred
    ///
    /// This strategy allows us to always make requests to the best available transports
    /// while keeping them available.
    async fn make_request(&self, req: RequestPacket) -> Result<ResponsePacket, TransportError> {
        // Get the top transports to use for this request
        let top_transports = {
            // Clone the vec, sort it, and take the top `self.active_transport_count`
            let mut transports_clone = (*self.transports).clone();
            transports_clone.sort_by(|a, b| b.cmp(a));
            transports_clone.into_iter().take(self.active_transport_count).collect::<Vec<_>>()
        };

        // Create a collection of future requests
        let mut futures = FuturesUnordered::new();

        // Launch requests to all active transports in parallel
        for transport in top_transports {
            let req_clone = req.clone();
            let mut transport_clone = transport.clone();

            let future = async move {
                let start = Instant::now();
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
}

impl<S> Service<RequestPacket> for FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Sync
        + Clone
        + Debug
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
}

impl FallbackLayer {
    /// Set the number of active transports to use (must be greater than 0)
    pub const fn with_active_transport_count(mut self, count: NonZeroUsize) -> Self {
        self.active_transport_count = count.get();
        self
    }
}

impl<S> Layer<Vec<S>> for FallbackLayer
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + Debug
        + 'static,
{
    type Service = FallbackService<S>;

    fn layer(&self, inner: Vec<S>) -> Self::Service {
        FallbackService::new(inner, self.active_transport_count)
    }
}

impl Default for FallbackLayer {
    fn default() -> Self {
        Self { active_transport_count: DEFAULT_ACTIVE_TRANSPORT_COUNT }
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
