//! CCIP Read (ERC-3668) support for `eth_call`.
//!
//! Gateway URLs in [`CcipReadRequest::urls`] come from the contract that emitted
//! `OffchainLookup` and are therefore untrusted. The default
//! [`HttpCcipReadGateway`] only requires the `https` scheme: it does **not**
//! block private, link-local, loopback, or cloud-metadata addresses (for
//! example `169.254.169.254` or `127.0.0.0/8`). Callers that need an allowlist,
//! blocklist, or resolved-IP check should supply a custom [`CcipReadGateway`].

use crate::Provider;
use alloy_eips::BlockId;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::TransactionInputKind;
use alloy_sol_types::{sol, SolCall, SolError, SolValue};
use alloy_transport::TransportError;
#[cfg(not(target_family = "wasm"))]
use futures::future::BoxFuture as CcipFuture;
#[cfg(target_family = "wasm")]
use futures::future::LocalBoxFuture as CcipFuture;
use futures::{stream, StreamExt};
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// ENSIP-21 local batch-gateway sentinel (`x-batch-gateway:true`).
///
/// If this value appears **anywhere** in [`CcipReadRequest::urls`], the client
/// handles the request as an ENSIP-21 local batch and does **not** try the other
/// URLs in that list. That prefers the local batch protocol over ERC-3668
/// HTTP fallback order when a resolver advertises both.
const BATCH_GATEWAY_SENTINEL: &str = "x-batch-gateway:true";
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
const HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

mod abi {
    use super::*;

    sol! {
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

/// Configuration for a CCIP Read call.
#[derive(Clone, Debug)]
pub struct CcipReadConfig {
    /// Maximum number of `OffchainLookup` redirects followed for one call.
    pub max_redirects: usize,
    /// Maximum number of requests accepted in one ENSIP-21 batch.
    pub max_batch_size: usize,
    /// Maximum aggregate gateway concurrency, including nested ENSIP-21 batches.
    pub max_concurrent_requests: usize,
    /// Maximum total gateway URL attempts and batch nodes reserved for one call.
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
    /// If `x-batch-gateway:true` is present anywhere in this list, HTTP URLs in
    /// the same list are not contacted. That prefers the ENSIP-21 local batch
    /// protocol over ERC-3668 HTTP fallback order when a resolver advertises both.
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

/// Executes individual CCIP Read HTTP gateway requests.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait CcipReadGateway: Send + Sync {
    /// Fetches one request, trying its URL templates in order.
    async fn request(
        &self,
        request: &CcipReadRequest,
        max_response_size: usize,
    ) -> Result<Bytes, CcipReadGatewayError>;
}

/// The default HTTPS implementation of [`CcipReadGateway`].
///
/// Available behind the `ccip-read-http` feature.
///
/// # URL trust
///
/// Templates in [`CcipReadRequest::urls`] are chosen by the callee contract.
/// This gateway checks only that each URL (and each redirect) uses `https`. It
/// will follow an `https` URL to a private, link-local, loopback, or cloud
/// metadata address. To constrain destinations, construct
/// [`HttpCcipReadGateway::new`] with a custom [`reqwest::Client`] or implement
/// [`CcipReadGateway`] with your own allowlist, blocklist, or DNS/IP policy.
#[derive(Clone, Debug)]
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
pub struct HttpCcipReadGateway {
    client: reqwest::Client,
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
impl Default for HttpCcipReadGateway {
    fn default() -> Self {
        #[cfg(not(target_family = "wasm"))]
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    attempt.stop()
                } else if attempt.url().scheme() == "https" {
                    attempt.follow()
                } else {
                    attempt.error("CCIP Read redirect URL must use HTTPS")
                }
            }))
            .build()
            .expect("default CCIP Read HTTP client configuration is valid");
        #[cfg(target_family = "wasm")]
        let client = reqwest::Client::new();
        Self { client }
    }
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
impl HttpCcipReadGateway {
    /// Creates a gateway handler using an existing HTTP client.
    ///
    /// The client's redirect policy is retained. It should reject redirects to
    /// non-HTTPS URLs.
    pub const fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(Serialize)]
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
struct GatewayRequestBody<'a> {
    sender: &'a str,
    data: &'a str,
}

#[derive(Deserialize)]
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
struct GatewayResponse {
    data: Bytes,
}

/// Outcome of classifying one HTTP gateway response.
#[derive(Debug)]
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
enum GatewayResponseOutcome {
    Data(Bytes),
    Fatal(CcipReadGatewayError),
    Retry(CcipReadGatewayError),
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn is_http_client_error(status: u16) -> bool {
    (400..500).contains(&status)
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn is_http_success(status: u16) -> bool {
    (200..300).contains(&status)
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn size_limit_error(status: u16) -> CcipReadGatewayError {
    CcipReadGatewayError::http(status, "gateway response exceeded configured size limit")
}

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn classify_content_length(
    status: u16,
    content_length: Option<u64>,
    max_response_size: usize,
) -> Option<GatewayResponseOutcome> {
    let length = content_length?;
    if length <= max_response_size as u64 {
        return None;
    }
    let error = size_limit_error(status);
    Some(if is_http_client_error(status) {
        GatewayResponseOutcome::Fatal(error)
    } else {
        GatewayResponseOutcome::Retry(error)
    })
}

/// Classifies a fully buffered gateway HTTP body.
///
/// Used after the response has been read (or when the advertised Content-Length
/// is absent and the streamed body itself must be size-checked).
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn classify_buffered_gateway_response(
    status: u16,
    content_type: &str,
    body: &[u8],
    max_response_size: usize,
) -> GatewayResponseOutcome {
    if body.len() > max_response_size {
        let error = size_limit_error(status);
        return if is_http_client_error(status) {
            GatewayResponseOutcome::Fatal(error)
        } else {
            GatewayResponseOutcome::Retry(error)
        };
    }
    if is_http_client_error(status) {
        return GatewayResponseOutcome::Fatal(CcipReadGatewayError::http(
            status,
            response_message(body),
        ));
    }
    if !is_http_success(status) {
        return GatewayResponseOutcome::Retry(CcipReadGatewayError::http(
            status,
            response_message(body),
        ));
    }
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    {
        return GatewayResponseOutcome::Retry(CcipReadGatewayError::http(
            status,
            "gateway response was not application/json",
        ));
    }
    match serde_json::from_slice::<GatewayResponse>(body) {
        Ok(response) => GatewayResponseOutcome::Data(response.data),
        Err(err) => GatewayResponseOutcome::Retry(CcipReadGatewayError::http(
            status,
            format!("invalid gateway response: {err}"),
        )),
    }
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
impl CcipReadGateway for HttpCcipReadGateway {
    async fn request(
        &self,
        request: &CcipReadRequest,
        max_response_size: usize,
    ) -> Result<Bytes, CcipReadGatewayError> {
        if request.urls.is_empty() {
            return Err(CcipReadGatewayError::new("OffchainLookup contained no gateway URLs"));
        }

        let sender = format!("{:#x}", request.sender);
        let data = format!("{}", request.data);
        let mut last_error = None;

        for template in &request.urls {
            let has_data_placeholder = template.contains("{data}");
            let url = template.replace("{sender}", &sender).replace("{data}", &data);
            let parsed = match reqwest::Url::parse(&url) {
                Ok(url) if url.scheme() == "https" => url,
                Ok(_) => {
                    last_error =
                        Some(CcipReadGatewayError::new("CCIP Read gateway URL must use HTTPS"));
                    continue;
                }
                Err(err) => {
                    last_error = Some(CcipReadGatewayError::new(format!(
                        "invalid CCIP Read gateway URL: {err}"
                    )));
                    continue;
                }
            };

            let response = if has_data_placeholder {
                self.client.get(parsed).timeout(HTTP_REQUEST_TIMEOUT).send().await
            } else {
                self.client
                    .post(parsed)
                    .timeout(HTTP_REQUEST_TIMEOUT)
                    .json(&GatewayRequestBody { sender: &sender, data: &data })
                    .send()
                    .await
            };

            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    last_error =
                        Some(CcipReadGatewayError::new(format!("gateway request failed: {err}")));
                    continue;
                }
            };

            let status = response.status().as_u16();
            if let Some(outcome) =
                classify_content_length(status, response.content_length(), max_response_size)
            {
                match outcome {
                    GatewayResponseOutcome::Fatal(error) => return Err(error),
                    GatewayResponseOutcome::Retry(error) => {
                        last_error = Some(error);
                        continue;
                    }
                    GatewayResponseOutcome::Data(_) => unreachable!(),
                }
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let mut body = Vec::new();
            let mut body_stream = response.bytes_stream();
            let read_error = loop {
                match body_stream.next().await {
                    Some(Ok(chunk)) if body.len() + chunk.len() <= max_response_size => {
                        body.extend_from_slice(&chunk);
                    }
                    Some(Ok(_)) => {
                        break Some(size_limit_error(status));
                    }
                    Some(Err(err)) => {
                        break Some(CcipReadGatewayError::new(format!(
                            "failed reading response: {err}"
                        )));
                    }
                    None => break None,
                }
            };
            if let Some(error) = read_error {
                if is_http_client_error(status) {
                    return Err(error);
                }
                last_error = Some(error);
                continue;
            }

            match classify_buffered_gateway_response(
                status,
                &content_type,
                &body,
                max_response_size,
            ) {
                GatewayResponseOutcome::Data(data) => return Ok(data),
                GatewayResponseOutcome::Fatal(error) => return Err(error),
                GatewayResponseOutcome::Retry(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| CcipReadGatewayError::new("all gateway URLs failed")))
    }
}

/// Placeholder gateway for WASI Preview 1, where the `reqwest` transport is unavailable.
#[derive(Clone, Copy, Debug, Default)]
#[cfg(all(feature = "ccip-read-http", target_os = "wasi", target_env = "p1"))]
pub struct HttpCcipReadGateway;

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg(all(feature = "ccip-read-http", target_os = "wasi", target_env = "p1"))]
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

#[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
fn response_message(body: &[u8]) -> String {
    const LIMIT: usize = 1_024;
    let body = &body[..body.len().min(LIMIT)];
    String::from_utf8_lossy(body).into_owned()
}

/// An executor for CCIP Read enabled `eth_call` requests.
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
}

impl<G: CcipReadGateway> CcipReadClient<G> {
    /// Executes a CCIP Read enabled call against the pending block.
    pub async fn call<P, N>(
        &self,
        provider: &P,
        transaction: N::TransactionRequest,
    ) -> Result<Bytes, CcipReadError>
    where
        P: Provider<N>,
        N: Network,
    {
        self.call_at(provider, transaction, BlockId::pending()).await
    }

    /// Executes a CCIP Read enabled call against `block`.
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
        let context = BatchContext {
            total_requests: Arc::new(AtomicUsize::new(0)),
            concurrency: Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_requests)),
            config: &self.config,
        };

        for redirects in 0..=self.config.max_redirects {
            match provider.call(transaction.clone()).block(block).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let Some(revert) =
                        extract_offchain_lookup(&error, self.config.max_revert_data_size)?
                    else {
                        return Err(CcipReadError::Transport(error));
                    };
                    if redirects == self.config.max_redirects {
                        return Err(CcipReadError::TooManyRedirects(self.config.max_redirects));
                    }

                    let lookup = abi::OffchainLookup::abi_decode(&revert)
                        .map_err(CcipReadError::InvalidOffchainLookup)?;
                    if lookup.sender != target {
                        return Err(CcipReadError::SenderMismatch {
                            sender: lookup.sender,
                            target,
                        });
                    }

                    let response = self
                        .fetch(
                            CcipReadRequest {
                                sender: lookup.sender,
                                urls: lookup.urls,
                                data: lookup.callData,
                            },
                            &context,
                        )
                        .await?;
                    let args = (response, lookup.extraData).abi_encode_params();
                    let mut callback = Vec::with_capacity(4 + args.len());
                    callback.extend_from_slice(lookup.callbackFunction.as_slice());
                    callback.extend_from_slice(&args);
                    // Keep `input` and `data` in sync. Some eth_call clients reject
                    // requests where both fields are set and disagree (e.g. Geth).
                    transaction.set_input_kind(callback, TransactionInputKind::Both);
                }
            }
        }

        unreachable!("redirect loop always returns")
    }

    fn fetch<'a>(
        &'a self,
        request: CcipReadRequest,
        context: &'a BatchContext<'a>,
    ) -> CcipFuture<'a, Result<Bytes, CcipReadError>> {
        Box::pin(async move {
            if context.config.max_concurrent_requests == 0 {
                return Err(CcipReadError::InvalidConfig(
                    "max_concurrent_requests must be greater than zero".into(),
                ));
            }
            let is_batch = request.urls.iter().any(|url| url == BATCH_GATEWAY_SENTINEL);
            if is_batch {
                reserve_requests(context, 1)?;
                self.local_batch(request.data, context).await
            } else {
                if request.urls.len() > context.config.max_gateway_urls {
                    return Err(CcipReadError::ResourceLimit(format!(
                        "gateway URL count {} exceeds limit {}",
                        request.urls.len(),
                        context.config.max_gateway_urls
                    )));
                }
                reserve_requests(context, request.urls.len().max(1))?;
                let _permit = context.concurrency.acquire().await.map_err(|_| {
                    CcipReadError::InvalidBatch("gateway concurrency limiter closed".into())
                })?;
                let response = self
                    .gateway
                    .request(&request, context.config.max_response_size)
                    .await
                    .map_err(CcipReadError::from)?;
                if response.len() > context.config.max_response_size {
                    return Err(CcipReadError::ResourceLimit(format!(
                        "gateway response is {} bytes; limit is {}",
                        response.len(),
                        context.config.max_response_size
                    )));
                }
                Ok(response)
            }
        })
    }

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
            let results = stream::iter(call.requests.into_iter().enumerate().map(
                |(index, request)| async move {
                    let result = self
                        .fetch(
                            CcipReadRequest {
                                sender: request.sender,
                                urls: request.urls,
                                data: request.data,
                            },
                            context,
                        )
                        .await;
                    (index, result)
                },
            ))
            .buffer_unordered(context.config.max_concurrent_requests)
            .collect::<Vec<_>>()
            .await;

            let mut ordered = Vec::with_capacity(results.len());
            ordered.resize_with(results.len(), || None);
            for (index, result) in results {
                ordered[index] = Some(result);
            }

            let mut failures = Vec::with_capacity(ordered.len());
            let mut responses = Vec::with_capacity(ordered.len());
            for result in ordered.into_iter().flatten() {
                match result {
                    Ok(response) => {
                        failures.push(false);
                        responses.push(response);
                    }
                    Err(error) => {
                        failures.push(true);
                        responses.push(encode_batch_error(&error));
                    }
                }
            }

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

fn extract_offchain_lookup(
    error: &TransportError,
    max_revert_data_size: usize,
) -> Result<Option<Bytes>, CcipReadError> {
    let Some(raw) = error.as_error_resp().and_then(|payload| payload.data.as_ref()) else {
        return Ok(None);
    };
    let max_json_size = max_revert_data_size.saturating_mul(2).saturating_add(4_096);
    if raw.get().len() > max_json_size {
        // Avoid parsing and allocating an oversized, untrusted JSON-RPC error. Since its selector
        // cannot be established within the configured bound, preserve it as the original
        // transport error rather than misclassifying it as CCIP Read.
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

struct BatchContext<'a> {
    total_requests: Arc<AtomicUsize>,
    concurrency: Arc<tokio::sync::Semaphore>,
    config: &'a CcipReadConfig,
}

fn reserve_requests(context: &BatchContext<'_>, count: usize) -> Result<(), CcipReadError> {
    if context
        .total_requests
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(count).filter(|next| *next <= context.config.max_total_requests)
        })
        .is_err()
    {
        return Err(CcipReadError::ResourceLimit(format!(
            "total gateway request budget of {} exceeded",
            context.config.max_total_requests
        )));
    }
    Ok(())
}

fn encode_batch_error(error: &CcipReadError) -> Bytes {
    if let CcipReadError::Gateway(error) = error {
        if let Some(status) = error.status {
            return abi::HttpError { status, message: error.message.clone() }.abi_encode().into();
        }
    }
    alloy_sol_types::Revert::from(error.to_string()).abi_encode().into()
}

/// Returns a process-wide default HTTP CCIP Read client.
///
/// Reuses a single [`reqwest::Client`] so keep-alive / HTTP2 pooling is shared across
/// [`ProviderCcipReadExt`] convenience calls.
#[cfg(feature = "ccip-read-http")]
pub fn shared_http_ccip_read_client() -> &'static CcipReadClient<HttpCcipReadGateway> {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<CcipReadClient<HttpCcipReadGateway>> = OnceLock::new();
    CLIENT.get_or_init(|| CcipReadClient::new(HttpCcipReadGateway::default()))
}

/// Extension methods for CCIP Read enabled provider calls.
///
/// Requires the `ccip-read-http` feature (uses the default HTTP gateway).
#[cfg(feature = "ccip-read-http")]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait ProviderCcipReadExt<N: Network>: Provider<N> {
    /// Executes an `eth_call`, following ERC-3668 redirects and ENSIP-21 batches.
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
        sync::{Mutex, PoisonError},
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

    #[tokio::test]
    async fn follows_offchain_lookup_and_calls_callback() {
        assert_eq!(abi::OffchainLookup::SELECTOR, [0x55, 0x6f, 0x18, 0x30]);

        let target = address!("1111111111111111111111111111111111111111");
        let call_data = bytes!("abcdef");
        let gateway_response = bytes!("deadbeef");
        let callback = fixed_bytes!("12345678");
        let extra_data = bytes!("010203");
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{sender}/{data}".into()],
            callData: call_data.clone(),
            callbackFunction: callback,
            extraData: extra_data,
        }
        .abi_encode()
        .into();

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        asserter.push_success(&bytes!("feed"));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::with_responses([Ok(gateway_response)]);
        let client = CcipReadClient::new(gateway.clone());

        let result =
            client.call(&provider, TransactionRequest::default().to(target)).await.unwrap();

        assert_eq!(result, bytes!("feed"));
        assert_eq!(
            gateway.requests(),
            vec![CcipReadRequest {
                sender: target,
                urls: vec!["https://example.test/{sender}/{data}".into()],
                data: call_data,
            }]
        );
    }

    #[tokio::test]
    async fn callback_keeps_input_and_data_in_sync() {
        let target = address!("1111111111111111111111111111111111111111");
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{data}".into()],
            callData: bytes!("abcdef"),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: bytes!("010203"),
        }
        .abi_encode()
        .into();

        let asserter = Asserter::new();
        asserter.push_failure(revert_error(revert));
        asserter.push_success(&bytes!("feed"));
        let provider = ProviderBuilder::new().connect_mocked_client(asserter);
        let gateway = MockGateway::with_responses([Ok(bytes!("deadbeef"))]);
        let client = CcipReadClient::new(gateway);

        // Requests with both `input` and `data` previously broke after the callback
        // replaced only `input`, leaving a stale `data` field.
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
        let revert: Bytes = abi::OffchainLookup {
            sender,
            urls: vec!["https://example.test/{data}".into()],
            callData: Bytes::new(),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: Bytes::new(),
        }
        .abi_encode()
        .into();

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
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{data}".into(); 9],
            callData: Bytes::new(),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: Bytes::new(),
        }
        .abi_encode()
        .into();

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
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{data}".into()],
            callData: Bytes::new(),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: Bytes::new(),
        }
        .abi_encode()
        .into();

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
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{data}".into()],
            callData: bytes!("01020304"),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: Bytes::new(),
        }
        .abi_encode()
        .into();

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
        let revert: Bytes = abi::OffchainLookup {
            sender: target,
            urls: vec!["https://example.test/{data}".into()],
            callData: Bytes::new(),
            callbackFunction: fixed_bytes!("12345678"),
            extraData: Bytes::new(),
        }
        .abi_encode()
        .into();

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
        let context = BatchContext {
            total_requests: Arc::new(AtomicUsize::new(0)),
            concurrency: Arc::new(tokio::sync::Semaphore::new(
                client.config().max_concurrent_requests,
            )),
            config: client.config(),
        };

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
        let context = BatchContext {
            total_requests: Arc::new(AtomicUsize::new(0)),
            concurrency: Arc::new(tokio::sync::Semaphore::new(
                client.config().max_concurrent_requests,
            )),
            config: client.config(),
        };

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
        let context = BatchContext {
            total_requests: Arc::new(AtomicUsize::new(0)),
            concurrency: Arc::new(tokio::sync::Semaphore::new(
                client.config().max_concurrent_requests,
            )),
            config: client.config(),
        };

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

    #[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
    fn outcome_message(outcome: &GatewayResponseOutcome) -> &str {
        match outcome {
            GatewayResponseOutcome::Data(_) => "",
            GatewayResponseOutcome::Fatal(error) | GatewayResponseOutcome::Retry(error) => {
                &error.message
            }
        }
    }

    #[test]
    #[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
    fn buffered_gateway_response_table() {
        let json_ok = br#"{"data":"0xdead"}"#;
        match classify_buffered_gateway_response(200, "application/json", json_ok, 1024) {
            GatewayResponseOutcome::Data(data) => assert_eq!(data, bytes!("dead")),
            other => panic!("expected data, got {other:?}"),
        }

        let oversized = vec![b'x'; 16];
        let outcome = classify_buffered_gateway_response(200, "application/json", &oversized, 8);
        assert!(matches!(outcome, GatewayResponseOutcome::Retry(_)), "{outcome:?}");
        assert!(outcome_message(&outcome).contains("size limit"));

        let outcome = classify_buffered_gateway_response(200, "text/plain", json_ok, 1024);
        assert!(matches!(outcome, GatewayResponseOutcome::Retry(_)));
        assert!(outcome_message(&outcome).contains("application/json"));

        let outcome =
            classify_buffered_gateway_response(200, "application/json", b"{\"data\":", 1024);
        assert!(matches!(outcome, GatewayResponseOutcome::Retry(_)));
        assert!(outcome_message(&outcome).contains("invalid gateway response"));

        let outcome =
            classify_buffered_gateway_response(200, "application/json", b"not json", 1024);
        assert!(matches!(outcome, GatewayResponseOutcome::Retry(_)));
        assert!(outcome_message(&outcome).contains("invalid gateway response"));
    }

    #[test]
    #[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
    fn content_length_precheck_rejects_oversized() {
        assert!(classify_content_length(200, Some(16), 8).is_some());
        assert!(classify_content_length(200, Some(8), 8).is_none());
        assert!(classify_content_length(200, None, 8).is_none());
    }

    #[tokio::test]
    #[cfg(all(feature = "ccip-read-http", not(all(target_os = "wasi", target_env = "p1"))))]
    async fn http_gateway_rejects_empty_http_and_invalid_urls() {
        let gateway = HttpCcipReadGateway::default();
        let sender = address!("1111111111111111111111111111111111111111");

        let empty = gateway
            .request(&CcipReadRequest { sender, urls: vec![], data: bytes!("") }, 1024)
            .await
            .unwrap_err();
        assert!(empty.message.contains("no gateway URLs"));

        let http = gateway
            .request(
                &CcipReadRequest {
                    sender,
                    urls: vec!["http://example.test/{sender}/{data}".into()],
                    data: bytes!(""),
                },
                1024,
            )
            .await
            .unwrap_err();
        assert!(http.message.contains("HTTPS"));

        let invalid = gateway
            .request(
                &CcipReadRequest { sender, urls: vec!["not a url".into()], data: bytes!("") },
                1024,
            )
            .await
            .unwrap_err();
        assert!(invalid.message.contains("invalid CCIP Read gateway URL"));
    }
}
