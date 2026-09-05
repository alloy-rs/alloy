//! CCIP Read (ERC-3668) support for `eth_call`.
//!
//! [`CcipReadClient`] executes an `eth_call` and, when the callee reverts with an
//! `OffchainLookup`, fetches the requested data from the advertised gateways through a
//! [`CcipReadGateway`] and re-invokes the callback until the call succeeds. Requests that
//! advertise the ENSIP-21 batch gateway sentinel (`x-batch-gateway:true`) are served locally by
//! fanning out the batched requests.
//!
//! Gateway URLs come from the contract that emitted the revert and are therefore untrusted. The
//! default HTTP gateway (`HttpCcipReadGateway`, behind the `ccip-read-http` feature) only checks
//! that each URL uses `http` or `https`; it does not block private, link-local, loopback, or
//! cloud-metadata addresses. Callers that need an allowlist, blocklist, or resolved-IP policy
//! should supply a custom [`CcipReadGateway`].

use crate::Provider;
use alloy_eips::BlockId;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::TransactionInputKind;
use alloy_sol_types::{SolCall, SolError, SolValue};
use alloy_transport::TransportError;
#[cfg(not(target_family = "wasm"))]
use futures::future::BoxFuture as CcipFuture;
#[cfg(target_family = "wasm")]
use futures::future::LocalBoxFuture as CcipFuture;
use futures::{stream, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
mod http;
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
pub use http::HttpCcipReadGateway;

/// ENSIP-21 local batch gateway sentinel.
///
/// If this value appears anywhere in [`CcipReadRequest::urls`], the request is served as a local
/// ENSIP-21 batch and the other URLs in that list are not contacted.
const BATCH_GATEWAY_SENTINEL: &str = "x-batch-gateway:true";

mod abi {
    alloy_sol_types::sol! {
        /// The ERC-3668 revert used to request offchain data.
        error OffchainLookup(
            address sender,
            string[] urls,
            bytes callData,
            bytes4 callbackFunction,
            bytes extraData
        );

        /// An HTTP error returned by an ENSIP-21 local batch gateway.
        error HttpError(uint16 status, string message);

        /// A request made through the ENSIP-21 batch gateway protocol.
        struct BatchGatewayRequest {
            address sender;
            string[] urls;
            bytes data;
        }

        /// The ENSIP-21 batch gateway entry point.
        function query(BatchGatewayRequest[] requests)
            external
            view
            returns (bool[] failures, bytes[] responses);
    }
}

/// Limits applied to a CCIP Read call.
#[derive(Clone, Debug)]
pub struct CcipReadConfig {
    /// Maximum number of `OffchainLookup` redirects followed for one call.
    pub max_redirects: usize,
    /// Maximum number of requests accepted in one ENSIP-21 batch.
    pub max_batch_size: usize,
    /// Maximum number of concurrent gateway requests, including nested ENSIP-21 batches.
    pub max_concurrent_requests: usize,
    /// Maximum number of gateway URL attempts and batch nodes budgeted for one call.
    pub max_total_requests: usize,
    /// Maximum number of fallback gateway URLs accepted in one request.
    pub max_gateway_urls: usize,
    /// Maximum accepted `OffchainLookup` revert data size in bytes.
    pub max_revert_data_size: usize,
    /// Maximum accepted gateway response size in bytes.
    pub max_response_size: usize,
}

impl Default for CcipReadConfig {
    fn default() -> Self {
        Self {
            max_redirects: 4,
            max_batch_size: 50,
            max_concurrent_requests: 4,
            max_total_requests: 100,
            max_gateway_urls: 8,
            max_revert_data_size: 1_048_576,
            max_response_size: 1_048_576,
        }
    }
}

/// A gateway request described by an ERC-3668 `OffchainLookup` revert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CcipReadRequest {
    /// The contract that emitted the revert.
    pub sender: Address,
    /// Gateway URL templates, in ERC-3668 fallback order.
    ///
    /// If `x-batch-gateway:true` is present anywhere in this list, the request is served as a
    /// local ENSIP-21 batch and the other URLs are not contacted.
    pub urls: Vec<String>,
    /// Data supplied by the reverting contract.
    pub data: Bytes,
}

/// An error returned while fetching data from a CCIP Read gateway.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct CcipReadGatewayError {
    /// HTTP status, when the failure came from an HTTP response.
    pub status: Option<u16>,
    /// A human-readable description of the failure.
    pub message: String,
}

impl CcipReadGatewayError {
    /// Creates an error without an HTTP status.
    pub fn new(message: impl Into<String>) -> Self {
        Self { status: None, message: message.into() }
    }

    /// Creates an error for an unsuccessful HTTP response.
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        Self { status: Some(status), message: message.into() }
    }
}

/// Errors produced while executing a CCIP Read call.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CcipReadError {
    /// The underlying `eth_call` failed without a valid `OffchainLookup` revert.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// CCIP Read cannot be used for a contract-creation call.
    #[error("CCIP Read requires an eth_call target")]
    MissingTarget,
    /// The revert's sender did not match the contract that was called.
    #[error("OffchainLookup sender {sender} does not match call target {target}")]
    SenderMismatch {
        /// Sender encoded in the revert.
        sender: Address,
        /// Target of the `eth_call`.
        target: Address,
    },
    /// The redirect limit was exceeded.
    #[error("CCIP Read redirect limit of {0} exceeded")]
    TooManyRedirects(usize),
    /// The revert data had the `OffchainLookup` selector but invalid ABI data.
    #[error("invalid OffchainLookup revert: {0}")]
    InvalidOffchainLookup(alloy_sol_types::Error),
    /// A gateway request failed.
    #[error("CCIP Read gateway request failed: {0}")]
    Gateway(#[from] CcipReadGatewayError),
    /// An ENSIP-21 batch request was malformed or exceeded configured limits.
    #[error("invalid ENSIP-21 batch request: {0}")]
    InvalidBatch(String),
    /// The CCIP Read client configuration is invalid.
    #[error("invalid CCIP Read configuration: {0}")]
    InvalidConfig(String),
    /// A CCIP Read resource limit was exceeded.
    #[error("CCIP Read resource limit exceeded: {0}")]
    ResourceLimit(String),
}

/// Fetches offchain data for CCIP Read requests.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait CcipReadGateway: Send + Sync {
    /// Fetches one request, trying its URL templates in order.
    ///
    /// Responses larger than `max_response_size` bytes must be rejected.
    async fn request(
        &self,
        request: &CcipReadRequest,
        max_response_size: usize,
    ) -> Result<Bytes, CcipReadGatewayError>;
}

/// Placeholder for the default HTTP gateway on WASI Preview 1, where `reqwest` is unavailable.
///
/// Every request fails; supply a custom [`CcipReadGateway`] instead.
#[derive(Clone, Copy, Debug, Default)]
#[cfg(all(feature = "ccip-read-http", target_os = "wasi", target_env = "p1"))]
pub struct HttpCcipReadGateway;

#[cfg(all(feature = "ccip-read-http", target_os = "wasi", target_env = "p1"))]
#[async_trait::async_trait(?Send)]
impl CcipReadGateway for HttpCcipReadGateway {
    async fn request(
        &self,
        _request: &CcipReadRequest,
        _max_response_size: usize,
    ) -> Result<Bytes, CcipReadGatewayError> {
        Err(CcipReadGatewayError::new(
            "the default CCIP Read HTTP gateway is unavailable on WASI Preview 1",
        ))
    }
}

/// Executes `eth_call` requests that follow ERC-3668 `OffchainLookup` reverts.
#[derive(Clone, Debug)]
pub struct CcipReadClient<G> {
    gateway: G,
    config: CcipReadConfig,
}

#[cfg(feature = "ccip-read-http")]
impl Default for CcipReadClient<HttpCcipReadGateway> {
    fn default() -> Self {
        Self::new(HttpCcipReadGateway::default())
    }
}

impl<G> CcipReadClient<G> {
    /// Creates a client with the default limits.
    pub fn new(gateway: G) -> Self {
        Self { gateway, config: CcipReadConfig::default() }
    }

    /// Sets the limits used by this client.
    pub const fn with_config(mut self, config: CcipReadConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the configured limits.
    pub const fn config(&self) -> &CcipReadConfig {
        &self.config
    }

    /// Returns the gateway used to fetch offchain data.
    pub const fn gateway(&self) -> &G {
        &self.gateway
    }
}

impl<G: CcipReadGateway> CcipReadClient<G> {
    /// Executes a CCIP Read enabled call against the latest block.
    pub async fn call<P, N>(
        &self,
        provider: &P,
        transaction: N::TransactionRequest,
    ) -> Result<Bytes, CcipReadError>
    where
        P: Provider<N>,
        N: Network,
    {
        self.call_at(provider, transaction, BlockId::latest()).await
    }

    /// Executes a CCIP Read enabled call against `block`.
    ///
    /// The initial call and every callback call are issued against `block`. With a block tag such
    /// as `latest`, the underlying state can move between those calls while gateway requests are
    /// in flight.
    pub async fn call_at<P, N>(
        &self,
        provider: &P,
        mut transaction: N::TransactionRequest,
        block: BlockId,
    ) -> Result<Bytes, CcipReadError>
    where
        P: Provider<N>,
        N: Network,
    {
        if self.config.max_concurrent_requests == 0 {
            return Err(CcipReadError::InvalidConfig(
                "max_concurrent_requests must be greater than zero".into(),
            ));
        }
        let target = transaction.to().ok_or(CcipReadError::MissingTarget)?;
        let context = BatchContext::new(&self.config);

        let mut redirects = 0;
        loop {
            let error = match provider.call(transaction.clone()).block(block).await {
                Ok(result) => return Ok(result),
                Err(error) => error,
            };
            let Some(revert) = extract_offchain_lookup(&error, self.config.max_revert_data_size)?
            else {
                return Err(CcipReadError::Transport(error));
            };
            if redirects == self.config.max_redirects {
                return Err(CcipReadError::TooManyRedirects(self.config.max_redirects));
            }
            redirects += 1;

            let lookup = abi::OffchainLookup::abi_decode(&revert)
                .map_err(CcipReadError::InvalidOffchainLookup)?;
            if lookup.sender != target {
                return Err(CcipReadError::SenderMismatch { sender: lookup.sender, target });
            }

            let request =
                CcipReadRequest { sender: lookup.sender, urls: lookup.urls, data: lookup.callData };
            let response = self.fetch(request, &context).await?;
            let mut callback = lookup.callbackFunction.to_vec();
            callback.extend_from_slice(&(response, lookup.extraData).abi_encode_params());
            // Keep `input` and `data` in sync: some nodes reject requests where both are set and
            // disagree.
            transaction.set_input_kind(callback, TransactionInputKind::Both);
        }
    }

    /// Fetches one request, serving ENSIP-21 batches locally.
    fn fetch<'a>(
        &'a self,
        request: CcipReadRequest,
        context: &'a BatchContext<'a>,
    ) -> CcipFuture<'a, Result<Bytes, CcipReadError>> {
        Box::pin(async move {
            if request.urls.iter().any(|url| url == BATCH_GATEWAY_SENTINEL) {
                context.reserve(1)?;
                return self.local_batch(request.data, context).await;
            }

            if request.urls.len() > context.config.max_gateway_urls {
                return Err(CcipReadError::ResourceLimit(format!(
                    "gateway URL count {} exceeds limit {}",
                    request.urls.len(),
                    context.config.max_gateway_urls
                )));
            }
            context.reserve(request.urls.len().max(1))?;
            let _permit =
                context.concurrency.acquire().await.expect("CCIP Read semaphore is never closed");
            let response = self.gateway.request(&request, context.config.max_response_size).await?;
            if response.len() > context.config.max_response_size {
                return Err(CcipReadError::ResourceLimit(format!(
                    "gateway response is {} bytes; limit is {}",
                    response.len(),
                    context.config.max_response_size
                )));
            }
            Ok(response)
        })
    }

    /// Serves an ENSIP-21 `query` batch by fetching each request and encoding the results.
    fn local_batch<'a>(
        &'a self,
        data: Bytes,
        context: &'a BatchContext<'a>,
    ) -> CcipFuture<'a, Result<Bytes, CcipReadError>> {
        Box::pin(async move {
            let call = abi::queryCall::abi_decode(&data)
                .map_err(|err| CcipReadError::InvalidBatch(err.to_string()))?;
            if call.requests.len() > context.config.max_batch_size {
                return Err(CcipReadError::InvalidBatch(format!(
                    "batch contains {} requests; limit is {}",
                    call.requests.len(),
                    context.config.max_batch_size
                )));
            }

            let requests = call.requests.into_iter().map(|request| {
                let request = CcipReadRequest {
                    sender: request.sender,
                    urls: request.urls,
                    data: request.data,
                };
                self.fetch(request, context)
            });
            let (failures, responses) = stream::iter(requests)
                .buffered(context.config.max_concurrent_requests)
                .map(|result| match result {
                    Ok(response) => (false, response),
                    Err(error) => (true, encode_batch_error(&error)),
                })
                .unzip::<_, _, Vec<_>, Vec<_>>()
                .await;

            let encoded: Bytes =
                abi::queryCall::abi_encode_returns(&abi::queryReturn { failures, responses })
                    .into();
            if encoded.len() > context.config.max_response_size {
                return Err(CcipReadError::ResourceLimit(format!(
                    "batch gateway response is {} bytes; limit is {}",
                    encoded.len(),
                    context.config.max_response_size
                )));
            }
            Ok(encoded)
        })
    }
}

/// Per-call state shared by all gateway requests, including nested ENSIP-21 batches.
struct BatchContext<'a> {
    config: &'a CcipReadConfig,
    total_requests: AtomicUsize,
    concurrency: Semaphore,
}

impl<'a> BatchContext<'a> {
    fn new(config: &'a CcipReadConfig) -> Self {
        Self {
            config,
            total_requests: AtomicUsize::new(0),
            concurrency: Semaphore::new(config.max_concurrent_requests),
        }
    }

    /// Reserves `count` gateway requests from the call's total budget.
    fn reserve(&self, count: usize) -> Result<(), CcipReadError> {
        let limit = self.config.max_total_requests;
        self.total_requests
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(count).filter(|next| *next <= limit)
            })
            .map(drop)
            .map_err(|_| {
                CcipReadError::ResourceLimit(format!(
                    "total gateway request budget of {limit} exceeded"
                ))
            })
    }
}

/// Extracts `OffchainLookup` revert data from an `eth_call` error.
///
/// Returns `Ok(None)` if the error is not an `OffchainLookup` revert, so that it can be surfaced
/// as the original transport error.
fn extract_offchain_lookup(
    error: &TransportError,
    max_revert_data_size: usize,
) -> Result<Option<Bytes>, CcipReadError> {
    let Some(raw) = error.as_error_resp().and_then(|payload| payload.data.as_ref()) else {
        return Ok(None);
    };
    // Avoid parsing an oversized, untrusted JSON-RPC error. Its selector cannot be established
    // within the configured bound, so it is preserved as the original transport error rather than
    // misclassified as CCIP Read.
    let max_json_size = max_revert_data_size.saturating_mul(2).saturating_add(4_096);
    if raw.get().len() > max_json_size {
        return Ok(None);
    }
    let Ok(value) = serde_json::from_str(raw.get()) else {
        return Ok(None);
    };
    let Some(data) = find_offchain_lookup(&value) else {
        return Ok(None);
    };
    if data.len() > max_revert_data_size {
        return Err(CcipReadError::ResourceLimit(format!(
            "OffchainLookup revert data is {} bytes; limit is {max_revert_data_size}",
            data.len()
        )));
    }
    Ok(Some(data))
}

/// Finds the first hex string carrying the `OffchainLookup` selector in a JSON-RPC error's data,
/// which nodes nest in different shapes.
fn find_offchain_lookup(value: &serde_json::Value) -> Option<Bytes> {
    match value {
        serde_json::Value::String(value) => {
            let data: Bytes = value.parse().ok()?;
            data.starts_with(abi::OffchainLookup::SELECTOR.as_slice()).then_some(data)
        }
        serde_json::Value::Object(values) => values.values().find_map(find_offchain_lookup),
        serde_json::Value::Array(values) => values.iter().find_map(find_offchain_lookup),
        _ => None,
    }
}

/// Encodes a failed batch request as ENSIP-21 error data.
fn encode_batch_error(error: &CcipReadError) -> Bytes {
    if let CcipReadError::Gateway(CcipReadGatewayError { status: Some(status), message }) = error {
        return abi::HttpError { status: *status, message: message.clone() }.abi_encode().into();
    }
    alloy_sol_types::Revert::from(error.to_string()).abi_encode().into()
}

/// Returns a process-wide default HTTP CCIP Read client.
///
/// This reuses a single [`reqwest::Client`] so that connections are pooled across
/// [`ProviderCcipReadExt`] calls.
#[cfg(feature = "ccip-read-http")]
pub fn shared_http_ccip_read_client() -> &'static CcipReadClient<HttpCcipReadGateway> {
    static CLIENT: std::sync::OnceLock<CcipReadClient<HttpCcipReadGateway>> =
        std::sync::OnceLock::new();
    CLIENT.get_or_init(CcipReadClient::default)
}

/// Extension trait for CCIP Read enabled `eth_call` requests using the default HTTP gateway.
///
/// See [`CcipReadClient`] to customize the gateway or the limits.
#[cfg(feature = "ccip-read-http")]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait ProviderCcipReadExt<N: Network>: Provider<N> {
    /// Executes an `eth_call` against the latest block, following ERC-3668 redirects and
    /// serving ENSIP-21 batches.
    async fn call_with_ccip_read(
        &self,
        transaction: N::TransactionRequest,
    ) -> Result<Bytes, CcipReadError>;

    /// Executes a CCIP Read enabled `eth_call` against `block`.
    async fn call_with_ccip_read_at(
        &self,
        transaction: N::TransactionRequest,
        block: BlockId,
    ) -> Result<Bytes, CcipReadError>;
}

#[cfg(feature = "ccip-read-http")]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<P, N> ProviderCcipReadExt<N> for P
where
    P: Provider<N>,
    N: Network,
{
    async fn call_with_ccip_read(
        &self,
        transaction: N::TransactionRequest,
    ) -> Result<Bytes, CcipReadError> {
        shared_http_ccip_read_client().call(self, transaction).await
    }

    async fn call_with_ccip_read_at(
        &self,
        transaction: N::TransactionRequest,
        block: BlockId,
    ) -> Result<Bytes, CcipReadError> {
        shared_http_ccip_read_client().call_at(self, transaction, block).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_json_rpc::ErrorPayload;
    use alloy_primitives::{address, bytes, fixed_bytes};
    use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};
    use alloy_transport::mock::Asserter;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex, PoisonError},
    };

    #[derive(Clone, Debug, Default)]
    struct MockGateway {
        responses: Arc<Mutex<VecDeque<Result<Bytes, CcipReadGatewayError>>>>,
        requests: Arc<Mutex<Vec<CcipReadRequest>>>,
    }

    impl MockGateway {
        fn with_responses(
            responses: impl IntoIterator<Item = Result<Bytes, CcipReadGatewayError>>,
        ) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses.into_iter().collect())),
                requests: Arc::default(),
            }
        }

        fn requests(&self) -> Vec<CcipReadRequest> {
            self.requests.lock().unwrap_or_else(PoisonError::into_inner).clone()
        }
    }

    #[async_trait::async_trait]
    impl CcipReadGateway for MockGateway {
        async fn request(
            &self,
            request: &CcipReadRequest,
            _max_response_size: usize,
        ) -> Result<Bytes, CcipReadGatewayError> {
            self.requests.lock().unwrap_or_else(PoisonError::into_inner).push(request.clone());
            self.responses
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .pop_front()
                .unwrap_or_else(|| Err(CcipReadGatewayError::new("no mock response")))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct BatchMockGateway;

    #[async_trait::async_trait]
    impl CcipReadGateway for BatchMockGateway {
        async fn request(
            &self,
            request: &CcipReadRequest,
            _max_response_size: usize,
        ) -> Result<Bytes, CcipReadGatewayError> {
            match request.data.as_ref() {
                [1] => Ok(bytes!("aaaa")),
                [2] => Err(CcipReadGatewayError::http(404, "not found")),
                _ => Err(CcipReadGatewayError::new("unexpected request")),
            }
        }
    }

    fn revert_error(data: Bytes) -> ErrorPayload {
        ErrorPayload::internal_error_with_message_and_obj(
            "call failed".into(),
            serde_json::value::to_raw_value(&data).unwrap(),
        )
    }

    fn offchain_lookup(sender: Address, urls: Vec<String>, call_data: Bytes) -> Bytes {
        abi::OffchainLookup {
            sender,
            urls,
            callData: call_data,
            callbackFunction: fixed_bytes!("12345678"),
            extraData: bytes!("010203"),
        }
        .abi_encode()
        .into()
    }

    #[tokio::test]
    async fn follows_offchain_lookup_and_calls_callback() {
        assert_eq!(abi::OffchainLookup::SELECTOR, [0x55, 0x6f, 0x18, 0x30]);

        let target = address!("1111111111111111111111111111111111111111");
        let call_data = bytes!("abcdef");
        let urls = vec!["https://example.test/{sender}/{data}".to_string()];

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(offchain_lookup(
            target,
            urls.clone(),
            call_data.clone(),
        )));
        asserter.push_success(&bytes!("feed"));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::with_responses([Ok(bytes!("deadbeef"))]);
        let client = CcipReadClient::new(gateway.clone());

        let result =
            client.call(&provider, TransactionRequest::default().to(target)).await.unwrap();

        assert_eq!(result, bytes!("feed"));
        assert_eq!(
            gateway.requests(),
            vec![CcipReadRequest { sender: target, urls, data: call_data }]
        );
    }

    #[tokio::test]
    async fn callback_keeps_input_and_data_in_sync() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert =
            offchain_lookup(target, vec!["https://example.test/{data}".into()], bytes!("abcdef"));

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        asserter.push_success(&bytes!("feed"));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::with_responses([Ok(bytes!("deadbeef"))]);
        let client = CcipReadClient::new(gateway);

        // Requests with both `input` and `data` set must have both replaced by the callback.
        let result = client
            .call(
                &provider,
                TransactionRequest::default()
                    .to(target)
                    .input(TransactionInput::both(bytes!("00"))),
            )
            .await
            .unwrap();

        assert_eq!(result, bytes!("feed"));
    }

    #[tokio::test]
    async fn rejects_sender_mismatch() {
        let target = address!("1111111111111111111111111111111111111111");
        let sender = address!("2222222222222222222222222222222222222222");
        let revert =
            offchain_lookup(sender, vec!["https://example.test/{data}".into()], Bytes::new());

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::default();

        let error = CcipReadClient::new(gateway.clone())
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CcipReadError::SenderMismatch { sender: actual_sender, target: actual_target }
                if actual_sender == sender && actual_target == target
        ));
        assert!(gateway.requests().is_empty());
    }

    #[tokio::test]
    async fn rejects_excessive_gateway_url_list() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert =
            offchain_lookup(target, vec!["https://example.test/{data}".into(); 9], Bytes::new());

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::default();

        let error = CcipReadClient::new(gateway.clone())
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(matches!(error, CcipReadError::ResourceLimit(message) if message.contains("URL")));
        assert!(gateway.requests().is_empty());
    }

    #[tokio::test]
    async fn enforces_response_limit_for_custom_gateways() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert =
            offchain_lookup(target, vec!["https://example.test/{data}".into()], Bytes::new());

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::with_responses([Ok(bytes!("0102"))]);
        let config = CcipReadConfig { max_response_size: 1, ..Default::default() };

        let error = CcipReadClient::new(gateway)
            .with_config(config)
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CcipReadError::ResourceLimit(message) if message.contains("response"))
        );
    }

    #[tokio::test]
    async fn rejects_oversized_revert_data_before_decoding() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert =
            offchain_lookup(target, vec!["https://example.test/{data}".into()], bytes!("01020304"));

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let config = CcipReadConfig { max_revert_data_size: 4, ..Default::default() };

        let error = CcipReadClient::new(MockGateway::default())
            .with_config(config)
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CcipReadError::ResourceLimit(message) if message.contains("revert"))
        );
    }

    #[tokio::test]
    async fn preserves_oversized_non_ccip_rpc_errors() {
        let target = address!("1111111111111111111111111111111111111111");
        let asserter = Asserter::new();
        asserter.push_failure(revert_error(vec![0u8; 5_000].into()));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let config = CcipReadConfig { max_revert_data_size: 1, ..Default::default() };

        let error = CcipReadClient::new(MockGateway::default())
            .with_config(config)
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(matches!(error, CcipReadError::Transport(_)));
    }

    #[tokio::test]
    async fn enforces_redirect_limit() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert =
            offchain_lookup(target, vec!["https://example.test/{data}".into()], Bytes::new());

        let asserter = Asserter::new();
        for _ in 0..3 {
            asserter.push_failure(revert_error(revert.clone()));
        }
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway =
            MockGateway::with_responses([Ok(bytes!("01")), Ok(bytes!("02")), Ok(bytes!("03"))]);
        let config = CcipReadConfig { max_redirects: 2, ..Default::default() };

        let error = CcipReadClient::new(gateway.clone())
            .with_config(config)
            .call(&provider, TransactionRequest::default().to(target))
            .await
            .unwrap_err();

        assert!(matches!(error, CcipReadError::TooManyRedirects(2)));
        assert_eq!(gateway.requests().len(), 2);
    }

    #[tokio::test]
    async fn executes_batch_gateway_requests_in_original_order() {
        assert_eq!(abi::queryCall::SELECTOR, [0xa7, 0x80, 0xba, 0xb6]);

        let sender = address!("1111111111111111111111111111111111111111");
        let batch = abi::queryCall {
            requests: vec![
                abi::BatchGatewayRequest {
                    sender,
                    urls: vec!["https://one.test".into()],
                    data: bytes!("01"),
                },
                abi::BatchGatewayRequest {
                    sender,
                    urls: vec!["https://two.test".into()],
                    data: bytes!("02"),
                },
            ],
        }
        .abi_encode()
        .into();
        let client = CcipReadClient::new(BatchMockGateway);
        let context = BatchContext::new(client.config());

        let encoded = client
            .fetch(
                CcipReadRequest { sender, urls: vec![BATCH_GATEWAY_SENTINEL.into()], data: batch },
                &context,
            )
            .await
            .unwrap();
        let decoded = abi::queryCall::abi_decode_returns(&encoded).unwrap();

        assert_eq!(decoded.failures, vec![false, true]);
        assert_eq!(decoded.responses[0], bytes!("aaaa"));
        let http_error = abi::HttpError::abi_decode(&decoded.responses[1]).unwrap();
        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.message, "not found");
    }

    #[tokio::test]
    async fn enforces_response_limit_for_local_batch() {
        let sender = address!("1111111111111111111111111111111111111111");
        let batch = abi::queryCall {
            requests: vec![abi::BatchGatewayRequest {
                sender,
                urls: vec!["https://one.test".into()],
                data: bytes!("01"),
            }],
        }
        .abi_encode()
        .into();
        // Encoded (failures=[false], responses=[["aaaa"]]) is larger than 8 bytes.
        let config = CcipReadConfig { max_response_size: 8, ..Default::default() };
        let client = CcipReadClient::new(BatchMockGateway).with_config(config);
        let context = BatchContext::new(client.config());

        let error = client
            .fetch(
                CcipReadRequest { sender, urls: vec![BATCH_GATEWAY_SENTINEL.into()], data: batch },
                &context,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CcipReadError::ResourceLimit(message) if message.contains("batch gateway response")
        ));
    }

    #[tokio::test]
    async fn limits_nested_batch_recursion() {
        let sender = address!("1111111111111111111111111111111111111111");
        let mut data = Bytes::new();
        for _ in 0..3 {
            data = abi::queryCall {
                requests: vec![abi::BatchGatewayRequest {
                    sender,
                    urls: vec![BATCH_GATEWAY_SENTINEL.into()],
                    data,
                }],
            }
            .abi_encode()
            .into();
        }
        let config = CcipReadConfig { max_total_requests: 2, ..Default::default() };
        let client = CcipReadClient::new(MockGateway::default()).with_config(config);
        let context = BatchContext::new(client.config());

        let encoded = client
            .fetch(
                CcipReadRequest { sender, urls: vec![BATCH_GATEWAY_SENTINEL.into()], data },
                &context,
            )
            .await
            .unwrap();
        let outer = abi::queryCall::abi_decode_returns(&encoded).unwrap();

        assert_eq!(outer.failures, vec![false]);
        let middle = abi::queryCall::abi_decode_returns(&outer.responses[0]).unwrap();
        assert_eq!(middle.failures, vec![true]);
        assert!(alloy_sol_types::Revert::abi_decode(&middle.responses[0])
            .unwrap()
            .reason
            .contains("budget"));
    }

    #[tokio::test]
    async fn rejects_zero_max_concurrent_requests_before_eth_call() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let client = CcipReadClient::new(MockGateway::default())
            .with_config(CcipReadConfig { max_concurrent_requests: 0, ..Default::default() });

        let error = client
            .call(
                &provider,
                TransactionRequest::default()
                    .to(address!("1111111111111111111111111111111111111111")),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(error, CcipReadError::InvalidConfig(message) if message.contains("max_concurrent_requests"))
        );
    }
}
