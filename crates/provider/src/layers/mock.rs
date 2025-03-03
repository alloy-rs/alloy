//! Mock Provider Layer

use std::{collections::VecDeque, sync::Arc};

use alloy_json_rpc::ErrorPayload;
use alloy_network::Network;
use alloy_primitives::U64;
use alloy_rpc_client::NoParams;
use alloy_transport::{TransportError, TransportErrorKind};
use parking_lot::RwLock;
use serde::Serialize;

use crate::{Provider, ProviderCall, ProviderLayer};

/// A mock provider layer that returns responses that have been pushed to the [`Asserter`].
#[derive(Debug, Clone)]
pub struct MockLayer {
    asserter: Asserter,
}

impl MockLayer {
    /// Instantiate a new mock layer with the given [`Asserter`].
    pub fn new(asserter: Asserter) -> Self {
        MockLayer { asserter }
    }
}

impl<P, N> ProviderLayer<P, N> for MockLayer
where
    P: Provider<N>,
    N: Network,
{
    type Provider = MockProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        MockProvider::new(inner, self.asserter.clone())
    }
}

/// Container for pushing responses into the [`MockProvider`].
#[derive(Debug, Clone, Default)]
pub struct Asserter {
    responses: Arc<RwLock<VecDeque<MockResponse>>>,
}

impl Asserter {
    /// Instantiate a new asserter.
    pub fn new() -> Self {
        Asserter::default()
    }

    /// Insert a successful response into the queue.
    pub fn push_success<R: Serialize>(&self, response: R) {
        self.responses
            .write()
            .push_back(MockResponse::Success(serde_json::to_value(response).unwrap()));
    }

    /// Push a server error payload into the queue.
    pub fn push_error(&self, error: ErrorPayload) {
        self.push_err(TransportError::err_resp(error));
    }

    /// Insert an error response into the queue.
    pub fn push_err(&self, err: TransportError) {
        self.responses.write().push_back(MockResponse::Err(err));
    }

    /// Pop front to get the next response from the queue.
    pub fn pop_response(&self) -> Option<MockResponse> {
        self.responses.write().pop_front()
    }
}

/// A mock response that can be pushed into the asserter.
#[derive(Debug)]
pub enum MockResponse {
    /// A successful response that will be deserialized into the expected type.
    Success(serde_json::Value),
    /// An error response.
    Err(TransportError),
}

/// A [`MockProvider`] error.
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    /// An error occurred while deserializing the response from asserter into the expected type.
    #[error("could not deserialize response {0}")]
    DeserError(String),
    /// The response queue is empty.
    #[error("empty response queue")]
    EmptyQueue,
}

/// A mock provider implementation that returns responses from the [`Asserter`].
#[derive(Debug, Clone)]
pub struct MockProvider<P: Provider<N>, N: Network> {
    /// Inner dummy provider.
    inner: P,
    /// The [`Asserter`] to which response are pushed using [`Asserter::push_success`].
    ///
    /// Responses are popped from the asserter in the order they were pushed.
    asserter: Asserter,
    _network: std::marker::PhantomData<N>,
}

impl<P, N> MockProvider<P, N>
where
    P: Provider<N>,
    N: Network,
{
    /// Instantiate a new mock provider.
    pub fn new(inner: P, asserter: Asserter) -> Self {
        MockProvider { inner, asserter, _network: std::marker::PhantomData }
    }
}

impl<P, N> Provider<N> for MockProvider<P, N>
where
    P: Provider<N>,
    N: Network,
{
    fn root(&self) -> &crate::RootProvider<N> {
        self.inner.root()
    }

    fn get_block_number(&self) -> ProviderCall<NoParams, U64, u64> {
        let value =
            self.asserter.pop_response().ok_or(TransportErrorKind::custom(MockError::EmptyQueue));

        let res = match value {
            Ok(MockResponse::Success(value)) => serde_json::from_value(value)
                .map_err(|e| TransportErrorKind::custom(MockError::DeserError(e.to_string()))),
            Ok(MockResponse::Err(err)) | Err(err) => Err(err),
        };

        ProviderCall::Ready(Some(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;

    #[tokio::test]
    async fn test_mock() {
        let (provider, asserter) = ProviderBuilder::mocked();

        asserter.push_success(21965802);
        asserter.push_success(21965803);
        asserter.push_err(TransportError::NullResp);

        let response = provider.get_block_number().await.unwrap();
        assert_eq!(response, 21965802);

        let response = provider.get_block_number().await.unwrap();
        assert_eq!(response, 21965803);

        let err_res = provider.get_block_number().await.unwrap_err();
        assert!(matches!(err_res, TransportError::NullResp));

        let response = provider.get_block_number().await.unwrap_err();

        assert!(response.to_string().contains("empty response queue"));
    }
}
