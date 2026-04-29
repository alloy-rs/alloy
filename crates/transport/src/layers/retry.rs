use crate::{
    error::{RpcErrorExt, TransportError, TransportErrorKind},
    TransportFut,
};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use core::fmt;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tower::{Layer, Service};
use tracing::trace;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

/// The default average cost of a request in Compute Units (CU).
const DEFAULT_AVG_COST: u64 = 20u64;

/// A Transport Layer that is responsible for retrying requests based on the
/// error type. See [`TransportError`].
///
/// TransportError: crate::error::TransportError
#[derive(Debug, Clone)]
pub struct RetryBackoffLayer<P: RetryPolicy = RateLimitRetryPolicy> {
    /// The maximum number of retries for rate limit errors.
    max_rate_limit_retries: u32,
    /// The initial backoff in milliseconds.
    initial_backoff: u64,
    /// The number of Compute Units per second for this provider.
    compute_units_per_second: u64,
    /// The average cost of a request. Defaults to [DEFAULT_AVG_COST].
    avg_cost: u64,
    /// The [RetryPolicy] to use. Defaults to [RateLimitRetryPolicy].
    policy: P,
}

impl RetryBackoffLayer {
    /// Creates a new retry layer with the given parameters and the default [RateLimitRetryPolicy].
    pub const fn new(
        max_rate_limit_retries: u32,
        initial_backoff: u64,
        compute_units_per_second: u64,
    ) -> Self {
        Self {
            max_rate_limit_retries,
            initial_backoff,
            compute_units_per_second,
            avg_cost: DEFAULT_AVG_COST,
            policy: RateLimitRetryPolicy,
        }
    }

    /// Sets the average Compute Unit (CU) cost per request. Defaults to `20` CU.
    ///
    /// Based on Alchemy’s published Compute Unit (CU) table, most frequently used
    /// JSON-RPC methods fall within the `10–20` CU range, with only a small number
    /// of higher-cost outliers (such as log queries or transaction submissions).
    /// Consequently, an average cost of `20` CU per request serves as a practical
    /// and representative estimate for typical EVM workloads
    ///
    /// Alchemy also uses this `20` CU figure when expressing throughput in
    /// requests per second. For example, the free tier maps `500 CU/s` to
    /// approximately `25 req/s` under this average, which aligns with the `20` CU.
    ///
    /// References:
    /// - <https://www.alchemy.com/docs/reference/compute-unit-costs#evm-standard-json-rpc-methods>
    /// - <https://www.alchemy.com/pricing#table-products>
    pub const fn with_avg_unit_cost(mut self, avg_cost: u64) -> Self {
        self.avg_cost = avg_cost;
        self
    }
}

impl<P: RetryPolicy> RetryBackoffLayer<P> {
    /// Creates a new retry layer with the given parameters and [RetryPolicy].
    pub const fn new_with_policy(
        max_rate_limit_retries: u32,
        initial_backoff: u64,
        compute_units_per_second: u64,
        policy: P,
    ) -> Self {
        Self {
            max_rate_limit_retries,
            initial_backoff,
            compute_units_per_second,
            policy,
            avg_cost: DEFAULT_AVG_COST,
        }
    }
}

/// [RateLimitRetryPolicy] implements [RetryPolicy] to determine whether to retry depending on the
/// err.
#[derive(Debug, Copy, Clone, Default)]
#[non_exhaustive]
pub struct RateLimitRetryPolicy;

impl RateLimitRetryPolicy {
    /// Creates a new [`RetryPolicy`] that in addition to this policy respects the given closure
    /// function for detecting if an error should be retried.
    pub fn or<F>(self, f: F) -> OrRetryPolicyFn<Self>
    where
        F: Fn(&TransportError) -> bool + Send + Sync + 'static,
    {
        OrRetryPolicyFn::new(self, f)
    }
}

/// [RetryPolicy] defines logic for which [TransportError] instances should
/// the client retry the request and try to recover from.
pub trait RetryPolicy: Send + Sync + std::fmt::Debug {
    /// Whether to retry the request based on the given `error`
    fn should_retry(&self, error: &TransportError) -> bool;

    /// Providers may include the `backoff` in the error response directly
    fn backoff_hint(&self, error: &TransportError) -> Option<std::time::Duration>;
}

impl RetryPolicy for RateLimitRetryPolicy {
    fn should_retry(&self, error: &TransportError) -> bool {
        error.is_retryable()
    }

    /// Provides a backoff hint if the error response contains it
    fn backoff_hint(&self, error: &TransportError) -> Option<std::time::Duration> {
        error.backoff_hint()
    }
}

/// A [`RetryPolicy`] that supports an additional closure for deciding if an error should be
/// retried.
#[derive(Clone)]
pub struct OrRetryPolicyFn<P = RateLimitRetryPolicy> {
    inner: Arc<dyn Fn(&TransportError) -> bool + Send + Sync>,
    base: P,
}

impl<P> OrRetryPolicyFn<P> {
    /// Creates a new instance with the given base policy and the given closure
    pub fn new<F>(base: P, or: F) -> Self
    where
        F: Fn(&TransportError) -> bool + Send + Sync + 'static,
    {
        Self { inner: Arc::new(or), base }
    }
}

impl<P: RetryPolicy> RetryPolicy for OrRetryPolicyFn<P> {
    fn should_retry(&self, error: &TransportError) -> bool {
        self.inner.as_ref()(error) || self.base.should_retry(error)
    }

    fn backoff_hint(&self, error: &TransportError) -> Option<Duration> {
        self.base.backoff_hint(error)
    }
}

impl<P: fmt::Debug> fmt::Debug for OrRetryPolicyFn<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrRetryPolicyFn")
            .field("base", &self.base)
            .field("inner", &"{{..}}")
            .finish_non_exhaustive()
    }
}

impl<S, P: RetryPolicy + Clone> Layer<S> for RetryBackoffLayer<P> {
    type Service = RetryBackoffService<S, P>;

    fn layer(&self, inner: S) -> Self::Service {
        RetryBackoffService {
            inner,
            policy: self.policy.clone(),
            max_rate_limit_retries: self.max_rate_limit_retries,
            initial_backoff: self.initial_backoff,
            compute_units_per_second: self.compute_units_per_second,
            requests_enqueued: Arc::new(AtomicU32::new(0)),
            avg_cost: self.avg_cost,
        }
    }
}

/// A Tower Service used by the RetryBackoffLayer that is responsible for retrying requests based
/// on the error type. See [TransportError] and [RateLimitRetryPolicy].
#[derive(Debug, Clone)]
pub struct RetryBackoffService<S, P: RetryPolicy = RateLimitRetryPolicy> {
    /// The inner service
    inner: S,
    /// The [RetryPolicy] to use.
    policy: P,
    /// The maximum number of retries for rate limit errors
    max_rate_limit_retries: u32,
    /// The initial backoff in milliseconds
    initial_backoff: u64,
    /// The number of compute units per second for this service
    compute_units_per_second: u64,
    /// The number of requests currently enqueued
    requests_enqueued: Arc<AtomicU32>,
    /// The average cost of a request.
    avg_cost: u64,
}

impl<S, P: RetryPolicy> RetryBackoffService<S, P> {
    const fn initial_backoff(&self) -> Duration {
        Duration::from_millis(self.initial_backoff)
    }
}

impl<S, P> Service<RequestPacket> for RetryBackoffService<S, P>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + 'static
        + Clone,
    P: RetryPolicy + Clone + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Our middleware doesn't care about backpressure, so it's ready as long
        // as the inner service is ready.
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: RequestPacket) -> Self::Future {
        let inner = self.inner.clone();
        let this = self.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);
        Box::pin(async move {
            let ahead_in_queue = this.requests_enqueued.fetch_add(1, Ordering::SeqCst) as u64;
            let mut rate_limit_retry_number: u32 = 0;
            loop {
                let err;
                let res = inner.call(request.clone()).await;

                match res {
                    Ok(res) => {
                        // Pick the trigger error that drives the retry decision below.
                        //  - Single: the response *is* the envelope, so collapse any error.
                        //  - Batch: a batch is one HTTP request, so the only knob this layer has is
                        //    "retry the whole batch or not" — retrying just one sub-call would mean
                        //    splitting the batch, which lives a layer up. Trigger a retry iff at
                        //    least one sub-call error is one the policy would actually retry on
                        //    (otherwise a revert mixed in would loop until max-retries). If nothing
                        //    is worth retrying, pass the batch through so each sub-call's result
                        //    reaches its own Waiter.
                        let trigger = match &res {
                            ResponsePacket::Single(s) => s.payload.as_error().cloned(),
                            ResponsePacket::Batch(items) => items
                                .iter()
                                .filter_map(|r| r.payload.as_error())
                                .find(|e| {
                                    this.policy
                                        .should_retry(&TransportError::ErrorResp((*e).clone()))
                                })
                                .cloned(),
                        };
                        if let Some(e) = trigger {
                            err = TransportError::ErrorResp(e);
                        } else {
                            this.requests_enqueued.fetch_sub(1, Ordering::SeqCst);
                            return Ok(res);
                        }
                    }
                    Err(e) => err = e,
                }

                let should_retry = this.policy.should_retry(&err);
                if should_retry {
                    rate_limit_retry_number += 1;
                    if rate_limit_retry_number > this.max_rate_limit_retries {
                        this.requests_enqueued.fetch_sub(1, Ordering::SeqCst);
                        return Err(TransportErrorKind::custom_str(&format!(
                            "Max retries exceeded {err}"
                        )));
                    }
                    trace!(%err, "retrying request");

                    let current_queued_reqs = this.requests_enqueued.load(Ordering::SeqCst) as u64;

                    // try to extract the requested backoff from the error or compute the next
                    // backoff based on retry count
                    let backoff_hint = this.policy.backoff_hint(&err);
                    let next_backoff = backoff_hint.unwrap_or_else(|| this.initial_backoff());

                    let seconds_to_wait_for_compute_budget = compute_unit_offset_in_secs(
                        this.avg_cost,
                        this.compute_units_per_second,
                        current_queued_reqs,
                        ahead_in_queue,
                    );
                    let total_backoff = next_backoff
                        + std::time::Duration::from_secs(seconds_to_wait_for_compute_budget);

                    trace!(
                        total_backoff_millis = total_backoff.as_millis(),
                        budget_backoff_millis = seconds_to_wait_for_compute_budget * 1000,
                        default_backoff_millis = next_backoff.as_millis(),
                        backoff_hint_millis = backoff_hint.map(|d| d.as_millis()),
                        "(all in ms) backing off due to rate limit"
                    );

                    sleep(total_backoff).await;
                } else {
                    this.requests_enqueued.fetch_sub(1, Ordering::SeqCst);
                    return Err(err);
                }
            }
        })
    }
}

/// Calculates an offset in seconds by taking into account the number of currently queued requests,
/// number of requests that were ahead in the queue when the request was first issued, the average
/// cost a weighted request (heuristic), and the number of available compute units per seconds.
///
/// Returns the number of seconds (the unit the remote endpoint measures compute budget) a request
/// is supposed to wait to not get rate limited. The budget per second is
/// `compute_units_per_second`, assuming an average cost of `avg_cost` this allows (in theory)
/// `compute_units_per_second / avg_cost` requests per seconds without getting rate limited.
/// By taking into account the number of concurrent request and the position in queue when the
/// request was first issued and determine the number of seconds a request is supposed to wait, if
/// at all
fn compute_unit_offset_in_secs(
    avg_cost: u64,
    compute_units_per_second: u64,
    current_queued_requests: u64,
    ahead_in_queue: u64,
) -> u64 {
    let request_capacity_per_second = compute_units_per_second.saturating_div(avg_cost).max(1);
    if current_queued_requests > request_capacity_per_second {
        current_queued_requests.min(ahead_in_queue).saturating_div(request_capacity_per_second)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{ErrorPayload, Id, Request, Response, ResponsePayload};
    use serde_json::value::RawValue;
    use std::sync::{
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
        Arc, Mutex,
    };

    #[test]
    fn test_compute_units_per_second() {
        let offset = compute_unit_offset_in_secs(17, 10, 0, 0);
        assert_eq!(offset, 0);
        let offset = compute_unit_offset_in_secs(17, 10, 2, 2);
        assert_eq!(offset, 2);
    }

    /// Tower mock that returns a queued [`ResponsePacket`] on each call and
    /// counts invocations. Lets the tests assert *exactly how many times*
    /// the retry layer hit the inner service.
    #[derive(Clone)]
    struct MockService {
        responses: Arc<Mutex<Vec<Result<ResponsePacket, TransportError>>>>,
        calls: Arc<AtomicUsize>,
    }

    impl MockService {
        fn new(responses: Vec<Result<ResponsePacket, TransportError>>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl Service<RequestPacket> for MockService {
        type Response = ResponsePacket;
        type Error = TransportError;
        type Future = TransportFut<'static>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: RequestPacket) -> Self::Future {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            // Pop the next canned response. Panicking on an unexpected extra
            // call is the point: any retry the layer performs trips the test.
            let next = self.responses.lock().unwrap().remove(0);
            Box::pin(async move { next })
        }
    }

    fn ok_response(id: u64, json_value: &str) -> Response {
        Response {
            id: Id::Number(id),
            payload: ResponsePayload::Success(
                RawValue::from_string(json_value.to_owned()).unwrap(),
            ),
        }
    }

    fn err_response(id: u64, code: i64, message: &'static str) -> Response {
        Response {
            id: Id::Number(id),
            payload: ResponsePayload::Failure(ErrorPayload {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    fn dummy_request() -> RequestPacket {
        RequestPacket::Single(Request::new("eth_call", Id::Number(1), ()).serialize().unwrap())
    }

    /// What we want the layer to deliver to the caller for a given mocked exchange.
    #[derive(Debug)]
    enum Expected {
        BatchPassThrough { successes: usize, errors: usize },
        SingleSuccess,
        ErrorResp(i64),
        MaxRetriesExceeded,
    }

    struct Case {
        name: &'static str,
        /// Responses the mock returns in FIFO order, one per `call`. The case
        /// asserts the layer consumed exactly `expected_calls` of them.
        responses: Vec<Result<ResponsePacket, TransportError>>,
        expected_calls: usize,
        expected: Expected,
    }

    impl Case {
        async fn run(self) -> Result<(), String> {
            let Self { responses, expected_calls, expected, .. } = self;
            let mock = MockService::new(responses);
            let calls = mock.calls.clone();
            // 2 retries, 1ms backoff, huge CU/s so the queue offset is 0.
            let mut svc = RetryBackoffLayer::new(2, 1, 1_000_000).layer(mock);
            let res = svc.call(dummy_request()).await;

            let actual_calls = calls.load(AtomicOrdering::SeqCst);
            if actual_calls != expected_calls {
                return Err(format!("calls: want {expected_calls}, got {actual_calls}"));
            }
            match (&expected, &res) {
                (Expected::BatchPassThrough { successes, errors }, Ok(packet)) => {
                    let batch = packet.as_batch().ok_or("want Batch, got Single")?;
                    let ok = batch.iter().filter(|r| r.is_success()).count();
                    let er = batch.iter().filter(|r| r.is_error()).count();
                    (ok == *successes && er == *errors).then_some(()).ok_or_else(|| {
                        format!("want {successes}ok/{errors}err, got {ok}ok/{er}err")
                    })
                }
                (Expected::SingleSuccess, Ok(packet)) => packet
                    .as_single()
                    .filter(|s| s.is_success())
                    .map(|_| ())
                    .ok_or_else(|| "want Single success".into()),
                (Expected::ErrorResp(want), Err(TransportError::ErrorResp(e))) => (e.code == *want)
                    .then_some(())
                    .ok_or_else(|| format!("want ErrorResp({want}), got ({})", e.code)),
                (Expected::MaxRetriesExceeded, Err(e)) => e
                    .to_string()
                    .contains("Max retries exceeded")
                    .then_some(())
                    .ok_or_else(|| format!("want 'Max retries exceeded', got: {e}")),
                (want, got) => Err(format!("shape: want {want:?}, got {got:?}")),
            }
        }
    }

    fn batch(responses: Vec<Response>) -> ResponsePacket {
        ResponsePacket::Batch(responses)
    }
    fn single(response: Response) -> ResponsePacket {
        ResponsePacket::Single(response)
    }

    /// Table test pinning down how `RetryBackoffLayer` handles every
    /// response shape we care about: single vs batch, crossed with
    /// all-ok / some-err / all-err and revert vs rate-limit error
    /// classes. Each row is a contract; the panic on failure lists
    /// every broken row so a regression names everything it touched.
    #[tokio::test]
    async fn retry_layer_response_handling() {
        let cases = vec![
            // ---- Single response: behavior is unchanged from upstream. ----
            Case {
                name: "single_success",
                responses: vec![Ok(single(ok_response(1, "\"0xdeadbeef\"")))],
                expected_calls: 1,
                expected: Expected::SingleSuccess,
            },
            Case {
                name: "single_revert_non_retryable",
                responses: vec![Ok(single(err_response(1, 3, "execution reverted")))],
                expected_calls: 1,
                expected: Expected::ErrorResp(3),
            },
            Case {
                name: "single_rate_limit_exhausts_retries",
                responses: vec![
                    Ok(single(err_response(1, 429, "rate limited"))),
                    Ok(single(err_response(1, 429, "rate limited"))),
                    Ok(single(err_response(1, 429, "rate limited"))),
                ],
                expected_calls: 3,
                expected: Expected::MaxRetriesExceeded,
            },
            // ---- Batch response: the new behavior under Option A. ----
            Case {
                name: "batch_all_success",
                responses: vec![Ok(batch(vec![
                    ok_response(1, "\"0x1\""),
                    ok_response(2, "\"0x2\""),
                ]))],
                expected_calls: 1,
                expected: Expected::BatchPassThrough { successes: 2, errors: 0 },
            },
            Case {
                name: "batch_one_revert_rest_success",
                responses: vec![Ok(batch(vec![
                    ok_response(1, "\"0x1\""),
                    err_response(2, 3, "execution reverted"),
                    ok_response(3, "\"0x3\""),
                ]))],
                expected_calls: 1,
                expected: Expected::BatchPassThrough { successes: 2, errors: 1 },
            },
            Case {
                name: "batch_some_revert_rest_success",
                responses: vec![Ok(batch(vec![
                    err_response(1, 3, "execution reverted"),
                    ok_response(2, "\"0x2\""),
                    err_response(3, 3, "execution reverted"),
                ]))],
                expected_calls: 1,
                expected: Expected::BatchPassThrough { successes: 1, errors: 2 },
            },
            Case {
                name: "batch_all_revert",
                responses: vec![Ok(batch(vec![
                    err_response(1, 3, "execution reverted"),
                    err_response(2, 3, "execution reverted"),
                ]))],
                expected_calls: 1,
                expected: Expected::BatchPassThrough { successes: 0, errors: 2 },
            },
            // ---- Hybrid behavior: any sub-call error the policy would
            // ---- retry on (rate-limit codes) triggers a retry of the
            // ---- whole batch — the only retry granularity available
            // ---- at this layer (a batch is one HTTP request).
            Case {
                name: "batch_all_rate_limit_retries_whole_batch",
                responses: vec![
                    Ok(batch(vec![
                        err_response(1, 429, "rate limited"),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        err_response(1, 429, "rate limited"),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        err_response(1, 429, "rate limited"),
                        err_response(2, 429, "rate limited"),
                    ])),
                ],
                expected_calls: 3,
                expected: Expected::MaxRetriesExceeded,
            },
            Case {
                name: "batch_mixed_rate_limit_and_success_retries_whole_batch",
                responses: vec![
                    Ok(batch(vec![
                        ok_response(1, "\"0x1\""),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        ok_response(1, "\"0x1\""),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        ok_response(1, "\"0x1\""),
                        err_response(2, 429, "rate limited"),
                    ])),
                ],
                expected_calls: 3,
                expected: Expected::MaxRetriesExceeded,
            },
            // Sibling: a non-retryable sub-call error (revert) mixed
            // with a retryable one (rate-limit). The rate-limit makes
            // the whole batch worth retrying; the revert hitches a ride
            // and gets re-executed, but that's the necessary cost of
            // batch-level retry granularity at this layer.
            Case {
                name: "batch_revert_plus_rate_limit_retries_whole_batch",
                responses: vec![
                    Ok(batch(vec![
                        err_response(1, 3, "execution reverted"),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        err_response(1, 3, "execution reverted"),
                        err_response(2, 429, "rate limited"),
                    ])),
                    Ok(batch(vec![
                        err_response(1, 3, "execution reverted"),
                        err_response(2, 429, "rate limited"),
                    ])),
                ],
                expected_calls: 3,
                expected: Expected::MaxRetriesExceeded,
            },
        ];

        let mut failures = Vec::new();
        for case in cases {
            let name = case.name;
            if let Err(why) = case.run().await {
                failures.push(format!("{name}: {why}"));
            }
        }
        assert!(
            failures.is_empty(),
            "{}/{} cases failed:\n  - {}",
            failures.len(),
            10,
            failures.join("\n  - ")
        );
    }
}
