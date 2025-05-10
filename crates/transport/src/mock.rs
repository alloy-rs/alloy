//! Mock transport and utility types.
//!
//! [`MockTransport`] returns responses that have been pushed into its associated [`Asserter`]'s
//! queue using FIFO.
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
/// See the [module documentation][self].
#[derive(Debug, Clone, Default)]
pub struct Asserter {
    responses: Arc<RwLock<VecDeque<MockResponse>>>,
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
            payload: self
                .asserter
                .pop_response()
                .ok_or_else(|| TransportErrorKind::custom_str("empty asserter response queue"))?,
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

// Tests are in `providers/tests/it/mock.rs`.
