//! Redundancy layer for blockchain client failover and response optimization.
//!
//! This module provides a transparent redundancy system that enables clients to:
//! - Query multiple providers concurrently for higher availability
//! - Implement failover strategies to handle provider outages
//! - Select optimal responses based on custom value functions
//! - Improve overall system reliability and performance
//!
//! ## Key Features
//!
//! - **First Success Strategy**: Returns the first successful response, minimizing latency
//! - **Highest Value Strategy**: Evaluates all responses with a custom function and returns the
//!   best one
//! - **Concurrent Execution**: All provider requests are made in parallel
//! - **Timeout Support**: Configurable timeouts to prevent hanging requests
//! - **Tower Layer Integration**: Seamlessly integrates with the Tower service ecosystem
//!
//! ## Use Cases
//!
//! Use this redundancy layer when you need:
//! - **High Availability**: Multiple RPC providers to ensure service continuity
//! - **Best Response Selection**: Custom logic to choose optimal responses (e.g., highest block
//!   number)
//! - **Latency Optimization**: Return the fastest successful response
//! - **Provider Failover**: Automatic fallback when primary providers fail
//!
//! ## Example
//!
//! ```rust,ignore
//! use tower::ServiceBuilder;
//! use crate::layers::redundancy::{RedundancyLayer, RedundancyStrategy};
//! use std::time::Duration;
//!
//! // Create redundancy layer with first-success strategy
//! let layer = RedundancyLayer::new(RedundancyStrategy::FirstRpcSuccess, Duration::from_secs(1));
//!
//! // Apply to multiple providers
//! let service = ServiceBuilder::new()
//!     .layer(layer)
//!     .service(vec![provider1, provider2, provider3]);
//! ```

use std::{
    fmt::{self, Debug},
    sync::{Arc, OnceLock},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{TransportError, TransportErrorKind, TransportFut};

use alloy_json_rpc::{RequestPacket, ResponsePacket, ResponsePayload, RpcError};
use futures::{stream::FuturesUnordered, StreamExt};
use serde_json::Value;
use tower::{Layer, Service};
use tracing::{error, span_enabled, trace, warn};

const TRACING_TARGET: &str = "redundancy_provider_request";

static VALUE_TO_F64: OnceLock<ValueConverter> = OnceLock::new();

/// A function that converts a JSON-RPC response value to a `f64` for comparison purposes.
pub type ValueConverter = Arc<dyn Fn(&Value) -> f64 + Send + Sync>;

/// Default value converter that converts a JSON-RPC tries to convert the `Value` to a `f64`,
/// otherwise returns `0.0`.
pub fn default_value_converter() -> ValueConverter {
    VALUE_TO_F64
        .get_or_init(|| {
            Arc::new(|value: &Value| {
                // Default implementation: convert to f64 if possible, otherwise return 0.0
                value.as_f64().unwrap_or(0.0)
            })
        })
        .clone()
}

/// Strategy for handling redundant service calls.
#[derive(Clone, Default)]
pub enum RedundancyStrategy {
    /// Return the first successful RPC response.
    #[default]
    FirstRpcSuccess,
    /// Return the response with the highest value according to the provided value function.
    HighestValue(ValueConverter),
}

impl Debug for RedundancyStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstRpcSuccess => write!(f, "FirstRpcSuccess"),
            Self::HighestValue(_) => write!(f, "HighestValue"),
        }
    }
}

/// Errors that can occur when using the redundancy service.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum RedundancyError {
    #[error("No requests provided")]
    NoRequestsProvided,
    #[error("No valid response was found")]
    NoValidResponse,
    #[error("All requests timed out after {0:?}")]
    AllTimedOut(Duration),
    #[error("All requests failed. Last error: {0}")]
    AllFailed(#[from] TransportError),
}

impl From<RedundancyError> for TransportError {
    fn from(err: RedundancyError) -> Self {
        match err {
            RedundancyError::AllFailed(e) => e,
            err => TransportErrorKind::custom(err),
        }
    }
}

/// Extension to control redundancy strategy per request
#[derive(Clone, Debug, Default)]
pub struct RedundancyExtension {
    /// The strategy to use for this request.
    pub strategy: RedundancyStrategy,
    /// Optional timeout for the request.
    pub timeout: Option<Duration>,
}

impl RedundancyExtension {
    /// Returns an extension with a default `HighestValue` strategy and unspecified timeout.
    pub fn default_highest_value() -> Self {
        Self {
            strategy: RedundancyStrategy::HighestValue(default_value_converter()),
            timeout: None,
        }
    }

    /// Returns an extension with a default `FirstSuccess` strategy and unspecified timeout.
    pub const fn default_first_success() -> Self {
        Self { strategy: RedundancyStrategy::FirstRpcSuccess, timeout: None }
    }
}

/// The [`RedundancyService`] consumes multiple providers and is able to
/// query them concurrently, returning responses according to the selected strategy.
///
/// The service ranks providers based on latency and stability metrics,
/// and will attempt to always use the best available providers.
#[derive(Clone, Debug)]
pub struct RedundancyService<S> {
    providers: Arc<Vec<S>>,
    /// To use if not specified in request extensions.
    default_strategy: RedundancyStrategy,
    /// To use if not specified in request extensions.
    default_timeout: Duration,
}

impl<S> RedundancyService<S> {
    /// Create a new redundancy service with the given providers, default strategy, and timeout.
    pub fn new(providers: Vec<S>, default_strategy: RedundancyStrategy, timeout: Duration) -> Self {
        Self { providers: Arc::new(providers), default_strategy, default_timeout: timeout }
    }
}

impl<S> Service<RequestPacket> for RedundancyService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Clone
        + Debug
        + Send
        + Sync
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
        Box::pin(async move { this.make_request(req).await.map_err(TransportError::from) })
    }
}

impl<S> RedundancyService<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Clone
        + Debug
        + Send
        + Sync
        + 'static,
{
    /// Make a request to the redundancy service.
    ///
    /// This method handles the core redundancy logic:
    /// 1. Gets the strategy from request extensions or uses default
    /// 2. Makes concurrent requests to all the providers
    /// 3. Processes responses according to the selected strategy
    async fn make_request(&self, req: RequestPacket) -> Result<ResponsePacket, RedundancyError> {
        let Some(first_request) = req.requests().first() else {
            return Err(RedundancyError::NoRequestsProvided);
        };

        let inspect_err = span_enabled!(target: TRACING_TARGET, tracing::Level::ERROR).then(|| {
            let method = first_request.method().to_owned();
            let params = first_request.params().map(ToOwned::to_owned);

            move |e: &RedundancyError| {
                error!(target: TRACING_TARGET,
                    "Error processing request {method} with params {params:?}: {e}",
                );
            }
        });

        // Get the strategy from extensions, or use the default
        let ext =
            first_request.meta().extensions().get::<RedundancyExtension>().cloned().unwrap_or(
                RedundancyExtension {
                    strategy: self.default_strategy.clone(),
                    ..Default::default()
                },
            );
        let timeout = ext.timeout.unwrap_or(self.default_timeout);

        // Launch requests to all active providers concurrently
        let futures = self.launch_requests(req);

        // Process responses according to strategy
        match ext.strategy {
            RedundancyStrategy::FirstRpcSuccess => handle_first_success(futures, timeout).await,
            RedundancyStrategy::HighestValue(fun) => {
                handle_highest_value(futures, fun, timeout).await
            }
        }
        .inspect_err(|e| {
            if let Some(inspect_err) = &inspect_err {
                inspect_err(e);
            }
        })
    }

    /// Launch concurrent requests to the selected providers
    fn launch_requests(&self, req: RequestPacket) -> FuturesUnordered<TransportFut<'static>> {
        let futures = FuturesUnordered::new();

        for provider in self.providers.iter() {
            let req_clone = req.clone();
            let mut provider_clone = provider.clone();

            let future = async move {
                let start = Instant::now();
                let result = provider_clone.call(req_clone).await;

                trace!(
                    target: TRACING_TARGET,
                    "Provider completed: latency={:?}, status={}",
                    start.elapsed(),
                    if result.is_ok() { "success" } else { "fail" }
                );

                result
            };

            // Enforce the `Send` bound by casting
            let pbf = Box::pin(future) as TransportFut<'static>;
            futures.push(pbf);
        }

        futures
    }
}

/// Redundancy layer for transparent provider failover. This layer will
/// consume a list of providers to provide better availability and
/// reliability.
///
/// The [`RedundancyService`] will attempt to make requests to multiple
/// providers concurrently, and return responses according to the selected strategy.
#[derive(Clone, Debug)]
pub struct RedundancyLayer {
    /// Default strategy to use if not specified in request extensions
    default_strategy: RedundancyStrategy,
    /// A timeout for requests made by the redundancy service.
    ///
    /// Wrapping this layer around a [timeout layer](https://docs.rs/tower-timeout/latest/tower_timeout/struct.TimeoutLayer.html)
    /// wouldn't allow to gather the "best" response we had for all the clients that responsed so
    /// far.
    timeout: Duration,
}

impl RedundancyLayer {
    /// Create a new redundancy layer with the given default strategy and timeout.
    pub const fn new(default_strategy: RedundancyStrategy, timeout: Duration) -> Self {
        Self { default_strategy, timeout }
    }
}

impl<S> Layer<Vec<S>> for RedundancyLayer
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Clone
        + Debug
        + Send
        + Sync
        + 'static,
{
    type Service = RedundancyService<S>;

    fn layer(&self, inner: Vec<S>) -> Self::Service {
        RedundancyService::new(inner, self.default_strategy.clone(), self.timeout)
    }
}

/// Handle responses using the [`RedundancyStrategy::FirstRpcSuccess`] strategy
async fn handle_first_success(
    futures: FuturesUnordered<TransportFut<'static>>,
    timeout: Duration,
) -> Result<ResponsePacket, RedundancyError> {
    let fut = wait_first_rpc_success(futures);

    // Hitting a timeout means all of the futures failed to complete in time,
    // so we can't return a valid response.
    tokio::time::timeout(timeout, fut).await.map_err(|_| RedundancyError::AllTimedOut(timeout))?
}

/// Handle responses using the [`RedundancyStrategy::HighestValue`] strategy
async fn handle_highest_value(
    futures: FuturesUnordered<TransportFut<'static>>,
    value_fn: ValueConverter,
    timeout: Duration,
) -> Result<ResponsePacket, RedundancyError> {
    // wait for all futures to complete, or timeout if specified. If the timeout
    // is reached, we can still return the best response we have so far.
    let results = wait_all_or_timeout(futures, timeout).await?;

    // Find the response with the highest value
    let mut best_response = None;
    let mut best_value = f64::NEG_INFINITY;

    for result in results {
        let Some(first_response) = result.responses().first() else {
            // TODO: handle batch request/response
            continue;
        };

        if let Ok(value) = serde_json::to_value(first_response) {
            let current_value = value_fn(&value);
            if current_value > best_value {
                best_value = current_value;
                best_response = Some(result);
            }
        }
    }

    best_response.ok_or(RedundancyError::NoValidResponse)
}

/// Wait for the first RPC success response from a list of futures
async fn wait_first_rpc_success(
    mut futs: FuturesUnordered<TransportFut<'static>>,
) -> Result<ResponsePacket, RedundancyError> {
    let mut last_error = None;

    while let Some(result) = futs.next().await {
        let response_packet = match result {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        if let ResponsePacket::Single(ref res) = response_packet {
            // Check if the response contains an error object.
            match res.payload {
                ResponsePayload::Failure(ref e) => {
                    last_error = Some(RpcError::ErrorResp(e.clone()));
                }
                ResponsePayload::Success(_) => {
                    return Ok(response_packet);
                }
            }
        } else {
            warn!("Batch response received, currently unsupported");
        }
    }

    Err(last_error.map(Into::into).expect("no error found"))
}

/// Wait for all futures to complete
///
/// If the timeout is reached, we return the results we have collected so far.
/// If all futures have completed, we return the results.
async fn wait_all_or_timeout(
    mut futs: FuturesUnordered<TransportFut<'static>>,
    timeout: Duration,
) -> Result<Vec<ResponsePacket>, RedundancyError> {
    let mut results = Vec::new();
    let mut last_error = None;

    let mut timeout_fut = Box::pin(tokio::time::sleep(timeout));

    loop {
        tokio::select! {
            _ = &mut timeout_fut => return Ok(results),
            result = futs.next() => {
                match result {
                    Some(Ok(response)) => {
                        if let ResponsePacket::Single(ref res) = response {
                            // Check if the response contains an error object.
                            match res.payload {
                                ResponsePayload::Failure(ref e) => {
                                    last_error = Some(RpcError::ErrorResp(e.clone()));
                                }
                                ResponsePayload::Success(_) => {
                                    results.push(response);
                                }
                            }
                        } else {
                            warn!("Batch response received, currently unsupported");
                        }
                    },
                    Some(Err(error)) => last_error = Some(error),
                    None => break,
                }
            }
        }
    }

    if results.is_empty() {
        Err(last_error.unwrap_or(RedundancyError::NoValidResponse.into()).into())
    } else {
        Ok(results)
    }
}
