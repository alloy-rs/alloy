//! Request lifecycle metadata and the shared recovery backoff.
use crate::time::Instant;
use alloy_json_rpc::{Id, Response};
use alloy_transport::TransportResult;
use rand::Rng;
use std::{fmt, future::Future, time::Duration};

tokio::task_local! {
    pub(crate) static REQUEST_DEADLINE: Instant;
}

/// Attach the provider's monotonic deadline to requests sent by this future.
/// This metadata stays local and never changes the JSON-RPC wire request.
pub async fn with_request_deadline<T>(deadline: Instant, future: impl Future<Output = T>) -> T {
    REQUEST_DEADLINE.scope(deadline, future).await
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
    use rand::{rngs::StdRng, SeedableRng};

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
