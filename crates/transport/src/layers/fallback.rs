use std::{
    collections::{BinaryHeap, VecDeque},
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use alloy_json_rpc::{Request, RequestPacket, ResponsePacket, SerializedRequest};
use futures::{stream::FuturesUnordered, StreamExt};
use tower::{Layer, Service};
use tracing::{debug, info};

use crate::{TransportError, TransportErrorKind, TransportFut};

// Constants for the ranking algorithm
const STABILITY_WEIGHT: f64 = 0.7;
const LATENCY_WEIGHT: f64 = 0.3;
const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_SAMPLE_COUNT: usize = 10;

/// Represents performance metrics for a transport.
#[derive(Debug)]
pub(crate) struct TransportMetrics {
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
    fn track_success(&mut self, duration: Duration, sample_count: usize) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_update = Instant::now();

        // Add to sample windows
        self.latencies.push_back(duration);
        self.successes.push_back(true);

        // Limit to sample count
        while self.latencies.len() > sample_count {
            self.latencies.pop_front();
        }
        while self.successes.len() > sample_count {
            self.successes.pop_front();
        }
    }

    fn track_failure(&mut self, sample_count: usize) {
        self.total_requests += 1;
        self.last_update = Instant::now();

        // Add to sample windows (no latency for failures)
        self.successes.push_back(false);

        // Limit to sample count
        while self.successes.len() > sample_count {
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
pub(crate) struct ScoredTransport<S> {
    id: usize, // Unique identifier for the transport
    transport: S,
    metrics: Arc<Mutex<TransportMetrics>>,
    sample_count: usize,
}

impl<S> ScoredTransport<S> {
    fn new(id: usize, transport: S, sample_count: usize) -> Self {
        Self { id, transport, metrics: Arc::new(Default::default()), sample_count }
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
    sample_count: usize,
    rank: bool,
}

impl<S> FallbackService<S>
where
    S: std::fmt::Debug,
{
    /// Create a new fallback provider from a list of transports.
    ///
    /// - The `active_transport_count` parameter controls how many
    ///   transports are used for requests at any one time.
    /// - The `sample_count` parameter controls how many samples are
    ///   used to calculate the score of each transport.
    /// - The `rank` parameter enables automatic transport ranking.
    pub fn new(
        transports: Vec<S>,
        active_transport_count: usize,
        sample_count: usize,
        rank: bool,
    ) -> Self {
        let scored_transports = transports
            .into_iter()
            .enumerate()
            .map(|(id, transport)| ScoredTransport::new(id, transport, sample_count))
            .collect::<Vec<_>>();

        Self {
            transports: Arc::new(Mutex::new(BinaryHeap::from(scored_transports))),
            active_transport_count,
            sample_count,
            rank,
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

        info!("Current transport rankings:");
        for (idx, transport) in sorted_transports.iter().enumerate() {
            info!("  #{}: Transport[{}] - {}", idx + 1, transport.id, transport.metrics_summary());
        }
    }
}

impl<S> FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static
        + std::fmt::Debug,
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
                    {
                        let mut metrics = transport.metrics.lock().expect("Lock poisoned");
                        metrics.track_success(duration, transport.sample_count);
                    } // Release the metrics lock

                    // Put all transports back in the heap
                    let mut transports = self.transports.lock().expect("Lock poisoned");
                    for transport in top_transports {
                        transports.push(transport);
                    }

                    // Log current rankings if ranking is enabled
                    if self.rank {
                        self.log_transport_rankings();
                    }

                    return Ok(response);
                }
                Err(error) => {
                    // Record failure
                    {
                        let mut metrics = transport.metrics.lock().expect("Lock poisoned");
                        metrics.track_failure(transport.sample_count);
                    } // Release the metrics lock

                    last_error = Some(error);
                }
            }
        }

        // If we got here, all transports failed.
        // Put all transports back in the heap
        let mut transports = self.transports.lock().expect("Lock poisoned");
        for transport in top_transports {
            transports.push(transport);
        }

        Err(last_error.unwrap_or_else(|| TransportErrorKind::custom_str("All transports failed")))
    }

    /// Start periodic health checks to rank transports
    fn start_health_check_task(&self, interval: Duration)
    where
        S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
            + Send
            + Clone
            + 'static,
    {
        let this = self.clone();
        tokio::spawn(async move {
            info!(
                "Starting health check task with interval: {:?}, sample count: {}",
                interval, this.sample_count
            );
            let mut tick = tokio::time::interval(interval);

            loop {
                tick.tick().await;
                debug!("Running transport health checks");

                // Get a copy of all transports
                let mut transports = Vec::new();
                {
                    let mut heap = this.transports.lock().expect("Lock poisoned");
                    while let Some(transport) = heap.pop() {
                        transports.push(transport);
                    }
                }

                // Ping each transport
                for transport in &mut transports {
                    let success = ping_transport(transport).await;
                    debug!(
                        "Health check for transport[{}]: {}",
                        transport.id,
                        if success { "success" } else { "failed" }
                    );
                }

                // Put all transports back in the heap (they will be automatically resorted)
                let mut heap = this.transports.lock().expect("Lock poisoned");
                for transport in transports {
                    heap.push(transport);
                }

                // Log current rankings
                this.log_transport_rankings();
            }
        });
    }
}

impl<S> Service<RequestPacket> for FallbackService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static
        + std::fmt::Debug,
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

/// Fallback layer for transparent transport failover.
///
/// This layer will transparently failover to a different transport if
/// the current transport fails.
///
/// The fallback service will attempt to make requests to multiple
/// transports in parallel, and return the first successful response.
///
/// If all transports fail, the fallback service will return an error.
///
/// # Automatic Transport Ranking
///
/// When ranking is enabled with `with_ranking(true)`, each transport is
/// automatically ranked based on latency & stability using a weighted
/// algorithm. By default:
///
/// - Every 10 seconds (configurable with `with_interval`), all transports
///   are pinged to measure their performance
/// - The past 10 samples (configurable with `with_sample_count`) are used
///   to calculate scores
/// - Stability (success rate) is weighted at 70%
/// - Latency (response time) is weighted at 30%
/// - The transport with the best combined score is prioritized first
///
/// Ranks are also automatically updated when transports are used,
/// so that the service can adapt to changing network conditions.
#[derive(Debug, Clone)]
pub struct FallbackLayer {
    active_transport_count: usize,
    /// Enable automatic transport ranking
    rank: bool,
    /// Interval for health checks (when ranking is enabled)
    interval: Duration,
    /// Number of samples to keep for calculation
    sample_count: usize,
}

impl FallbackLayer {
    /// Create a new fallback layer.
    ///
    /// The `active_transport_count` parameter controls how many
    /// transports are used for requests at any one time.
    pub fn new(active_transport_count: usize) -> Self {
        Self {
            active_transport_count,
            rank: false,
            interval: DEFAULT_INTERVAL,
            sample_count: DEFAULT_SAMPLE_COUNT,
        }
    }

    /// Enable or disable automatic transport ranking
    pub fn with_ranking(mut self, enabled: bool) -> Self {
        self.rank = enabled;
        self
    }

    /// Set the interval for health checks when ranking is enabled
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set the number of samples to keep for score calculation
    pub fn with_sample_count(mut self, count: usize) -> Self {
        self.sample_count = count;
        self
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
        let service =
            FallbackService::new(inner, self.active_transport_count, self.sample_count, self.rank);

        // If ranking is enabled, start the health check task
        if self.rank {
            service.start_health_check_task(self.interval);
        }

        service
    }
}

/// Health check ping to test transport latency and availability
async fn ping_transport<T>(transport: &mut ScoredTransport<T>) -> bool
where
    T: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + Clone
        + 'static,
{
    // Create a simple ping request
    let ping_id = format!("ping-{}", current_timestamp_ms());
    let ping_req = RequestPacket::from(
        SerializedRequest::try_from(Request::new("net_version", ping_id.into(), ()))
            .expect("valid serialization"),
    );

    let start = Instant::now();
    match transport.call(ping_req).await {
        Ok(_) => {
            let duration = start.elapsed();
            let mut metrics = transport.metrics.lock().expect("Lock poisoned");
            metrics.track_success(duration, transport.sample_count);
            true
        }
        Err(_) => {
            let mut metrics = transport.metrics.lock().expect("Lock poisoned");
            metrics.track_failure(transport.sample_count);
            false
        }
    }
}

/// Returns the current timestamp since the UNIX epoch in milliseconds.
fn current_timestamp_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64
}
