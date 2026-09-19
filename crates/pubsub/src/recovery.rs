//! Request lifecycle metadata and the shared recovery backoff.
use crate::time::Instant;
use alloy_json_rpc::{Id, Response};
use alloy_transport::TransportResult;
use rand::Rng;
use std::{
    collections::HashMap,
    fmt,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

tokio::task_local! {
    pub(crate) static REQUEST_TIMINGS: RequestTimings;
}

#[derive(Debug)]
struct RequestTiming {
    deadline: Instant,
    received_at: Option<Instant>,
}

/// Local timing evidence for one physical attempt, indexed by JSON-RPC ID.
///
/// Pubsub responses arrive independently. Their arrival times must survive
/// packet aggregation so a completed item does not expire while a sibling waits.
/// Create fresh metadata for each attempt; retries must not reuse arrival times.
#[derive(Debug, Clone)]
pub struct RequestTimings(Arc<Mutex<HashMap<Id, RequestTiming>>>);

impl RequestTimings {
    /// Set each item's deadline, including the physical attempt timeout.
    pub fn new(deadlines: impl IntoIterator<Item = (Id, Instant)>) -> Self {
        Self(Arc::new(Mutex::new(
            deadlines
                .into_iter()
                .map(|(id, deadline)| (id, RequestTiming { deadline, received_at: None }))
                .collect(),
        )))
    }

    /// Return the deadline supplied for this ID.
    pub fn deadline(&self, id: &Id) -> Option<Instant> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(id)
            .map(|timing| timing.deadline)
    }

    /// Return when the connection manager accepted this item's response.
    /// Missing entries do not establish that a response arrived before expiry.
    pub fn received_at(&self, id: &Id) -> Option<Instant> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(id)
            .and_then(|timing| timing.received_at)
    }

    pub(crate) fn record_response(&self, id: &Id, received_at: Instant) {
        if let Some(timing) =
            self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_mut(id)
        {
            timing.received_at = Some(received_at);
        }
    }
}

/// Attach per-item deadlines and arrival metadata to requests sent by this future.
/// The metadata stays local and never changes the JSON-RPC wire request.
pub async fn with_request_timings<T>(
    timings: RequestTimings,
    future: impl Future<Output = T>,
) -> T {
    REQUEST_TIMINGS.scope(timings, future).await
}

/// Capped exponential backoff shared by read recovery and connection recovery.
#[derive(Debug, Clone)]
pub struct RecoveryBackoff {
    ceiling_ms: u64,
}

impl Default for RecoveryBackoff {
    fn default() -> Self {
        Self { ceiling_ms: 100 }
    }
}

impl RecoveryBackoff {
    /// Sample between half and all of the current ceiling, then double the
    /// next ceiling up to 500 ms.
    pub fn next_delay(&mut self) -> Duration {
        self.sample_delay(&mut rand::thread_rng())
    }

    fn sample_delay(&mut self, rng: &mut impl Rng) -> Duration {
        let delay = rng.gen_range(self.ceiling_ms / 2..=self.ceiling_ms);
        self.ceiling_ms = (self.ceiling_ms * 2).min(500);
        Duration::from_millis(delay)
    }
}

/// A batch can contain completed responses alongside lost connections.
/// Retaining each ID lets the provider retry only unresolved reads.
#[derive(Debug)]
pub struct PartialBatchError(pub Vec<(Id, TransportResult<Response>)>);

impl fmt::Display for PartialBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RPC batch contains {} item results", self.0.len())
    }
}
impl std::error::Error for PartialBatchError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::InFlight;
    use alloy_json_rpc::Request;
    use rand::{rngs::StdRng, SeedableRng};

    #[tokio::test(start_paused = true)]
    async fn batch_items_keep_distinct_deadlines_and_original_receipt_times() {
        let started = Instant::now();
        let early = Id::Number(11);
        let late = Id::Number(22);
        let timings = RequestTimings::new([
            (early.clone(), started + Duration::from_millis(5)),
            (late.clone(), started + Duration::from_millis(20)),
        ]);
        with_request_timings(timings.clone(), async {
            let (first, first_rx) = InFlight::new(
                Request::new("eth_call", early.clone(), ("0x11", "0x123")).serialize().unwrap(),
                16,
            );
            let (second, second_rx) = InFlight::new(
                Request::new("eth_call", late.clone(), ("0x22", "0x456")).serialize().unwrap(),
                16,
            );
            assert_eq!(first.deadline, Some(started + Duration::from_millis(5)));
            assert_eq!(second.deadline, Some(started + Duration::from_millis(20)));
            let response = |id| Response {
                id,
                payload: alloy_json_rpc::ResponsePayload::Success(
                    serde_json::value::to_raw_value("0x42").unwrap(),
                ),
            };
            first.fulfill(response(early.clone()));
            tokio::time::sleep(Duration::from_millis(10)).await;
            second.fulfill(response(late.clone()));
            assert_eq!(first_rx.await.unwrap().unwrap().id, early);
            assert_eq!(second_rx.await.unwrap().unwrap().id, late);
        })
        .await;
        assert_eq!(timings.received_at(&early), Some(started));
        assert_eq!(timings.received_at(&late), Some(started + Duration::from_millis(10)));
    }

    #[tokio::test(start_paused = true)]
    async fn expired_and_cancelled_reads_do_not_record_successful_receipts() {
        let started = Instant::now();
        let timings = RequestTimings::new([
            (Id::Number(1), started + Duration::from_millis(5)),
            (Id::Number(2), started + Duration::from_millis(20)),
        ]);
        with_request_timings(timings.clone(), async {
            let (expired, expired_rx) = InFlight::new(
                Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap(),
                16,
            );
            let (cancelled, cancelled_rx) = InFlight::new(
                Request::new("eth_blockNumber", Id::Number(2), ()).serialize().unwrap(),
                16,
            );
            drop(cancelled_rx);
            tokio::time::sleep(Duration::from_millis(5)).await;
            for request in [expired, cancelled] {
                let id = request.request.id().clone();
                request.fulfill(Response {
                    id,
                    payload: alloy_json_rpc::ResponsePayload::Success(
                        serde_json::value::to_raw_value("0x42").unwrap(),
                    ),
                });
            }
            assert!(expired_rx.await.is_err());
        })
        .await;
        assert_eq!(timings.received_at(&Id::Number(1)), None);
        assert_eq!(timings.received_at(&Id::Number(2)), None);
    }

    #[test]
    fn capped_exponential_backoff_obeys_both_jitter_bounds() {
        let mut rng = StdRng::seed_from_u64(113);
        for _ in 0..1000 {
            let mut backoff = RecoveryBackoff::default();
            for ceiling in [100, 200, 400, 500, 500, 500] {
                let delay = backoff.sample_delay(&mut rng);
                assert!((Duration::from_millis(ceiling / 2)..=Duration::from_millis(ceiling))
                    .contains(&delay));
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn seeded_callers_follow_different_bounded_schedules() {
        async fn caller(seed: u64) -> Vec<Duration> {
            let start = Instant::now();
            let mut backoff = RecoveryBackoff::default();
            let mut rng = StdRng::seed_from_u64(seed);
            let mut times = Vec::new();
            for _ in 0..6 {
                let delay = backoff.sample_delay(&mut rng);
                tokio::time::sleep(delay).await;
                times.push(start.elapsed());
            }
            times
        }
        let (first, second) = tokio::join!(caller(113), caller(114));
        assert_ne!(first, second);
        for times in [first, second] {
            let mut previous = Duration::ZERO;
            for (at, ceiling) in times.into_iter().zip([100, 200, 400, 500, 500, 500]) {
                let delay = at - previous;
                assert!((Duration::from_millis(ceiling / 2)..=Duration::from_millis(ceiling + 1))
                    .contains(&delay));
                previous = at;
            }
        }
    }
}
