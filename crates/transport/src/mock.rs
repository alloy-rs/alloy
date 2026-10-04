//! Mock transport and utility types.
//!
//! [`MockTransport`] returns responses that have been pushed into its associated [`Asserter`]'s
//! queue using FIFO, and records every request it receives so that tests can assert what was
//! sent.
//!
//! # Examples
//!
//! ```ignore (dependency cycle)
//! use alloy_transport::mock::*;
//!
//! let asserter = Asserter::new();
//! let provider = ProviderBuilder::new()
//!     /* ... */
//!     .on_mocked_client(asserter.clone());
//!
//! let n = 12345;
//! asserter.push_success(&n);
//! let actual = provider.get_block_number().await.unwrap();
//! assert_eq!(actual, n);
//!
//! asserter.push_success(&U256::from(1));
//! provider.get_balance(Address::ZERO).await.unwrap();
//!
//! let requests = asserter.take_requests();
//! assert_eq!(requests[0].method(), "eth_blockNumber");
//! assert_eq!(requests[1].method(), "eth_getBalance");
//! assert_eq!(
//!     requests[1].params().unwrap().get(),
//!     r#"["0x0000000000000000000000000000000000000000","latest"]"#
//! );
//! ```

use crate::{TransportErrorKind, TransportResult};
use alloy_json_rpc as j;
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::VecDeque,
    sync::{Arc, PoisonError, RwLock},
};

/// A mock response that can be pushed into an [`Asserter`].
pub type MockResponse = j::ResponsePayload;

/// Container for pushing responses into a [`MockTransport`].
///
/// Mock responses are stored and returned with a FIFO queue.
///
/// Requests received by the [`MockTransport`] are recorded in a second FIFO queue, see
/// [`requests`](Self::requests).
///
/// See the [module documentation][self].
#[derive(Debug, Clone, Default)]
pub struct Asserter {
    responses: Arc<RwLock<VecDeque<MockResponse>>>,
    requests: Arc<RwLock<VecDeque<j::SerializedRequest>>>,
}

impl Asserter {
    /// Instantiate a new asserter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a response into the queue.
    pub fn push(&self, response: MockResponse) {
        self.write_q().push_back(response);
    }

    /// Insert a successful response into the queue.
    ///
    /// # Panics
    ///
    /// Panics if serialization fails.
    #[track_caller]
    pub fn push_success<R: Serialize>(&self, response: &R) {
        let s = serde_json::to_string(response).unwrap();
        self.push(MockResponse::Success(serde_json::value::RawValue::from_string(s).unwrap()));
    }

    /// Push an error payload into the queue.
    pub fn push_failure(&self, error: j::ErrorPayload) {
        self.push(MockResponse::Failure(error));
    }

    /// Push an internal error message into the queue.
    pub fn push_failure_msg(&self, msg: impl Into<Cow<'static, str>>) {
        self.push_failure(j::ErrorPayload::internal_error_message(msg.into()));
    }

    /// Pops the next mock response.
    pub fn pop_response(&self) -> Option<MockResponse> {
        self.write_q().pop_front()
    }

    /// Returns a read lock guard to the responses queue.
    pub fn read_q(&self) -> impl std::ops::Deref<Target = VecDeque<MockResponse>> + '_ {
        self.responses.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Returns a write lock guard to the responses queue.
    pub fn write_q(&self) -> impl std::ops::DerefMut<Target = VecDeque<MockResponse>> + '_ {
        self.responses.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Returns a copy of the recorded requests, oldest first.
    ///
    /// The [`MockTransport`] records every request it handles before popping the matching
    /// response, including requests that fail because the response queue is empty. Requests of a
    /// batch are recorded individually, in batch order.
    ///
    /// Recorded requests are kept until they are removed with [`pop_request`](Self::pop_request)
    /// or [`take_requests`](Self::take_requests).
    pub fn requests(&self) -> Vec<j::SerializedRequest> {
        self.requests.read().unwrap_or_else(PoisonError::into_inner).iter().cloned().collect()
    }

    /// Removes and returns the oldest recorded request.
    ///
    /// See [`requests`](Self::requests).
    pub fn pop_request(&self) -> Option<j::SerializedRequest> {
        self.write_requests().pop_front()
    }

    /// Removes and returns all recorded requests, oldest first.
    ///
    /// See [`requests`](Self::requests).
    pub fn take_requests(&self) -> Vec<j::SerializedRequest> {
        self.write_requests().drain(..).collect()
    }

    fn write_requests(
        &self,
    ) -> impl std::ops::DerefMut<Target = VecDeque<j::SerializedRequest>> + '_ {
        self.requests.write().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A transport that returns responses from an associated [`Asserter`].
///
/// See the [module documentation][self].
#[derive(Clone, Debug)]
pub struct MockTransport {
    asserter: Asserter,
}

impl MockTransport {
    /// Create a new [`MockTransport`] with the given [`Asserter`].
    pub const fn new(asserter: Asserter) -> Self {
        Self { asserter }
    }

    /// Return a reference to the associated [`Asserter`].
    pub const fn asserter(&self) -> &Asserter {
        &self.asserter
    }

    async fn handle(self, req: j::RequestPacket) -> TransportResult<j::ResponsePacket> {
        self.asserter.write_requests().extend(req.requests().iter().cloned());
        Ok(match req {
            j::RequestPacket::Single(req) => j::ResponsePacket::Single(self.map_request(req)?),
            j::RequestPacket::Batch(reqs) => j::ResponsePacket::Batch(
                reqs.into_iter()
                    .map(|req| self.map_request(req))
                    .collect::<TransportResult<_>>()?,
            ),
        })
    }

    fn map_request(&self, req: j::SerializedRequest) -> TransportResult<j::Response> {
        Ok(j::Response {
            id: req.id().clone(),
            payload: self.asserter.pop_response().ok_or_else(|| {
                TransportErrorKind::custom_str(&format!(
                    "empty asserter response queue for request with id {id} and method {method}",
                    id = req.id(),
                    method = req.method()
                ))
            })?,
        })
    }
}

impl std::ops::Deref for MockTransport {
    type Target = Asserter;

    fn deref(&self) -> &Self::Target {
        &self.asserter
    }
}

impl tower::Service<j::RequestPacket> for MockTransport {
    type Response = j::ResponsePacket;
    type Error = crate::TransportError;
    type Future = crate::TransportFut<'static>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: j::RequestPacket) -> Self::Future {
        Box::pin(self.clone().handle(req))
    }
}

// Tests are in `crates/provider/tests/it/mock.rs`.
