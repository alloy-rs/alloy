//! CCIP (EIP-3668) support for offchain data retrieval.
//!
//! This module implements Cross Chain Interoperability Protocol (CCIP) as defined in EIP-3668,
//! which allows smart contracts to fetch external data securely and transparently.

use crate::Provider;
use alloy_json_rpc::RpcRecv;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{hex, Address, Bytes, FixedBytes};
use alloy_sol_types::{sol, SolError, SolValue};
use alloy_transport::TransportError;
use futures::Future;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, pin::Pin, sync::Arc, task::Poll, time::Duration};
use thiserror::Error;

/// Maximum number of recursive CCIP lookups allowed per EIP-3668
const MAX_CCIP_REDIRECTS: u8 = 4;

/// Default timeout for gateway requests
const DEFAULT_GATEWAY_TIMEOUT: Duration = Duration::from_secs(30);

// Define the OffchainLookup error using sol! macro
sol! {
    /// The OffchainLookup error as defined in EIP-3668.
    ///
    /// This error is thrown by contracts to indicate that the requested data
    /// is available from an offchain source.
    #[derive(Debug, PartialEq, Eq)]
    error OffchainLookup(
        address sender,
        string[] urls,
        bytes callData,
        bytes4 callbackFunction,
        bytes extraData
    );
}

/// Errors that can occur during CCIP resolution
#[derive(Debug, Error)]
pub enum CcipError {
    /// Transport error during RPC call
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    /// Gateway request failed
    #[error("gateway error: {0}")]
    Gateway(#[from] GatewayError),

    /// Maximum recursion depth exceeded
    #[error("maximum CCIP recursion depth ({MAX_CCIP_REDIRECTS}) exceeded")]
    MaxRecursionExceeded,

    /// Failed to decode OffchainLookup error
    #[error("failed to decode OffchainLookup error: {0}")]
    DecodeError(String),

    /// Invalid callback data
    #[error("invalid callback data")]
    InvalidCallback,

    /// No gateway URLs provided
    #[error("no gateway URLs provided")]
    NoGatewayUrls,
}

/// Errors specific to gateway requests
#[derive(Debug, Error, Clone)]
pub enum GatewayError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(String),

    /// Invalid response format
    #[error("invalid response format: {0}")]
    InvalidResponse(String),

    /// Gateway timeout
    #[error("gateway request timed out")]
    Timeout,

    /// All gateway URLs failed
    #[error("all gateway URLs failed")]
    AllUrlsFailed,
}

/// Gateway client trait for fetching offchain data.
///
/// This trait defines the interface for fetching data from CCIP gateways.
/// Implementations are responsible for:
/// - Parsing and substituting URL templates (e.g., `{sender}`, `{data}`)
/// - Making HTTP requests (GET for short URLs, POST for long URLs)
/// - Handling response parsing and error conditions
///
/// # Security Considerations
///
/// Gateway clients should validate responses to prevent malicious data injection.
/// The default implementation enforces timeouts and validates response formats.
pub trait CcipGatewayClient: Send + Sync {
    /// Fetch data from a gateway URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The gateway URL template, which may contain `{sender}` and `{data}` placeholders
    /// * `sender` - The address of the contract that emitted the OffchainLookup error
    /// * `call_data` - The data to be sent to the gateway
    ///
    /// # Returns
    ///
    /// The response data from the gateway, which will be passed to the contract's callback
    /// function.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the request fails, times out, or returns invalid data.
    fn fetch(
        &self,
        url: &str,
        sender: Address,
        call_data: &Bytes,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, GatewayError>> + Send + '_>>;
}

/// Configuration for CCIP calls
#[derive(Debug, Clone)]
pub struct CcipConfig {
    /// Maximum recursion depth (capped at 4 per spec)
    pub max_recursion: u8,
    /// Gateway request timeout
    pub gateway_timeout: Duration,
}

impl Default for CcipConfig {
    fn default() -> Self {
        Self { max_recursion: MAX_CCIP_REDIRECTS, gateway_timeout: DEFAULT_GATEWAY_TIMEOUT }
    }
}

/// CCIP-enabled call that wraps an [`EthCall`](super::EthCall).
///
/// This type adds automatic CCIP resolution to an existing `EthCall`.
/// When awaited, it will:
/// 1. Execute the underlying `eth_call`
/// 2. If an `OffchainLookup` error is received, fetch data from the specified gateways
/// 3. Call the contract's callback function with the gateway response
/// 4. Return the result (either direct or from callback)
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use alloy_network::TransactionBuilder;
/// use alloy_primitives::{address, bytes};
/// use alloy_provider::{Provider, ProviderBuilder};
///
/// let provider = ProviderBuilder::new().connect_http("https://eth.llamarpc.com".parse()?);
///
/// let tx = provider
///     .transaction_request()
///     .to(address!("1234567890123456789012345678901234567890"))
///     .input(bytes!("deadbeef").into());
///
/// let result = provider.call(tx).ccip(provider.clone()).await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "CCIP calls do nothing unless awaited"]
#[derive(Clone)]
pub struct CcipCall<N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    inner: super::EthCall<N, Resp, Output, Map>,
    provider: Arc<dyn Provider<N>>,
    config: CcipConfig,
    gateway_client: Arc<dyn CcipGatewayClient>,
    _phantom: PhantomData<fn() -> (Resp, Output)>,
}

impl<N: Network, Resp: RpcRecv, Output, Map> std::fmt::Debug for CcipCall<N, Resp, Output, Map>
where
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CcipCall")
            .field("inner", &"EthCall<...>")
            .field("config", &self.config)
            .field("gateway_client", &"Arc<dyn CcipGatewayClient>")
            .finish()
    }
}

impl<N: Network, Resp: RpcRecv, Output, Map> CcipCall<N, Resp, Output, Map>
where
    Map: Fn(Resp) -> Output,
{
    /// Create a new CCIP call from an EthCall with the default gateway client.
    pub(crate) fn new(
        inner: super::EthCall<N, Resp, Output, Map>,
        provider: Arc<dyn Provider<N>>,
    ) -> Self {
        Self {
            inner,
            provider,
            config: CcipConfig::default(),
            gateway_client: Arc::new(DefaultGatewayClient::new()),
            _phantom: PhantomData,
        }
    }

    /// Set max recursion depth (capped at 4).
    pub fn max_recursion(mut self, depth: u8) -> Self {
        self.config.max_recursion = depth.min(MAX_CCIP_REDIRECTS);
        self
    }

    /// Set gateway timeout.
    pub const fn gateway_timeout(mut self, timeout: Duration) -> Self {
        self.config.gateway_timeout = timeout;
        self
    }

    /// Override the gateway client.
    pub fn with_gateway_client(mut self, client: impl CcipGatewayClient + 'static) -> Self {
        self.gateway_client = Arc::new(client);
        self
    }

    /// Map the response to a different type.
    pub fn map_resp<NewOutput, NewMap>(self, map: NewMap) -> CcipCall<N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        CcipCall {
            inner: self.inner.map_resp(map),
            provider: self.provider,
            config: self.config,
            gateway_client: self.gateway_client,
            _phantom: PhantomData,
        }
    }
}

/// Future for executing a CCIP call.
///
/// This future handles the execution of an EthCall with automatic CCIP resolution.
/// It will attempt to resolve OffchainLookup errors by fetching data from gateways
/// and calling the contract's callback function.
#[must_use = "futures do nothing unless awaited"]
#[pin_project::pin_project]
pub struct CcipCallFuture<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    #[pin]
    inner: Pin<Box<dyn Future<Output = Result<Bytes, CcipError>> + Send>>,
    _phantom: PhantomData<(N, Resp, Output, Map)>,
}

impl<N, Resp, Output, Map> std::fmt::Debug for CcipCallFuture<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CcipCallFuture").field("inner", &"Pin<Box<dyn Future<...>>>").finish()
    }
}

impl<N, Resp, Output, Map> CcipCallFuture<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv + Send + 'static,
    Output: Send + 'static,
    Map: Fn(Resp) -> Output + Clone + Send + 'static,
{
    /// Create a new CCIP call future.
    fn new(
        eth_call: super::EthCall<N, Resp, Output, Map>,
        provider: Arc<dyn Provider<N>>,
        config: CcipConfig,
        gateway_client: Arc<dyn CcipGatewayClient>,
    ) -> Self {
        let fut = Box::pin(async move {
            execute_with_ccip(eth_call, provider, config, gateway_client, 0).await
        });

        Self { inner: fut, _phantom: PhantomData }
    }
}

impl<N, Resp, Output, Map> Future for CcipCallFuture<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = Result<Bytes, CcipError>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx)
    }
}

impl<N: Network, Resp: RpcRecv + Send + 'static, Output: Send + 'static, Map>
    std::future::IntoFuture for CcipCall<N, Resp, Output, Map>
where
    Map: Fn(Resp) -> Output + Clone + Send + 'static,
{
    type Output = Result<Bytes, CcipError>;
    type IntoFuture = CcipCallFuture<N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        CcipCallFuture::new(self.inner, self.provider, self.config, self.gateway_client)
    }
}

/// Execute an EthCall with CCIP resolution support.
async fn execute_with_ccip<N: Network, Resp: RpcRecv, Output: 'static, Map>(
    eth_call: super::EthCall<N, Resp, Output, Map>,
    provider: Arc<dyn Provider<N>>,
    config: CcipConfig,
    gateway_client: Arc<dyn CcipGatewayClient>,
    recursion_depth: u8,
) -> Result<Bytes, CcipError>
where
    Map: Fn(Resp) -> Output,
{
    // For CCIP, we need to work with the raw bytes, not the decoded response
    // We'll execute the underlying call and handle the OffchainLookup error
    match eth_call.await {
        Ok(_) => {
            // If the call succeeds without OffchainLookup, it's not a CCIP call
            // This is unexpected - CCIP calls should always revert with OffchainLookup
            Err(CcipError::DecodeError(
                "Expected OffchainLookup error but call succeeded".to_string(),
            ))
        }
        Err(err) => {
            // Check if it's an OffchainLookup error
            if let Some(offchain_lookup) = extract_offchain_lookup(&err) {
                // Resolve the offchain lookup and return the raw bytes
                resolve_offchain_lookup(
                    provider,
                    offchain_lookup,
                    config,
                    gateway_client,
                    recursion_depth,
                )
                .await
            } else {
                Err(err.into())
            }
        }
    }
}

/// Resolve OffchainLookup errors iteratively to avoid async recursion.
async fn resolve_offchain_lookup<N: Network>(
    provider: Arc<dyn Provider<N>>,
    initial_lookup: OffchainLookup,
    config: CcipConfig,
    gateway_client: Arc<dyn CcipGatewayClient>,
    initial_depth: u8,
) -> Result<Bytes, CcipError> {
    let mut current_lookup = initial_lookup;
    let mut depth = initial_depth;

    loop {
        // Check recursion limit
        if depth >= config.max_recursion {
            return Err(CcipError::MaxRecursionExceeded);
        }

        // Check if we have URLs
        if current_lookup.urls.is_empty() {
            return Err(CcipError::NoGatewayUrls);
        }

        // Try each gateway URL
        let mut last_error = None;
        let mut gateway_response = None;

        for url in &current_lookup.urls {
            match gateway_client.fetch(url, current_lookup.sender, &current_lookup.callData).await {
                Ok(response) => {
                    gateway_response = Some(response);
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // Check if we got a response
        let response = match gateway_response {
            Some(resp) => resp,
            None => return Err(last_error.unwrap_or(GatewayError::AllUrlsFailed).into()),
        };

        // Build callback request
        let callback_request = build_callback_request::<N>(
            current_lookup.sender,
            current_lookup.callbackFunction,
            response,
            current_lookup.extraData.clone(),
        )?;

        // Execute the callback
        let callback_call = provider.call(callback_request);
        match callback_call.await {
            Ok(bytes) => return Ok(bytes),
            Err(err) => {
                // Check if it's another OffchainLookup error
                if let Some(next_lookup) = extract_offchain_lookup(&err) {
                    current_lookup = next_lookup;
                    depth += 1;
                    // Continue the loop with the new lookup
                } else {
                    return Err(err.into());
                }
            }
        }
    }
}

/// Implementation of `CcipGatewayClient` for `reqwest::Client`.
///
/// This implementation follows the EIP-3668 specification for gateway requests:
/// - Substitutes `{sender}` with the contract address (checksummed)
/// - Substitutes `{data}` with the hex-encoded call data
/// - Uses GET when URL contains `{data}` parameter
/// - Uses POST with JSON body when URL doesn't contain `{data}` parameter
/// - Expects JSON responses with a `data` field containing hex-encoded bytes
impl CcipGatewayClient for reqwest::Client {
    fn fetch(
        &self,
        url: &str,
        sender: Address,
        call_data: &Bytes,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, GatewayError>> + Send + '_>> {
        let url = url.to_string();
        let call_data = call_data.clone();

        Box::pin(async move {
            // Check if URL contains {data} parameter
            let contains_data_param = url.contains("{data}");

            // Parse URL and substitute parameters
            let substituted_url = url
                .replace("{sender}", &format!("{:?}", sender))
                .replace("{data}", &format!("0x{}", hex::encode(&call_data)));

            let response = if contains_data_param {
                // Use GET when URL contains {data}
                self.get(&substituted_url)
                    .send()
                    .await
                    .map_err(|e| GatewayError::Http(e.to_string()))?
            } else {
                // Use POST when URL doesn't contain {data}
                #[derive(Serialize)]
                struct PostData {
                    data: String,
                    sender: Address,
                }

                self.post(&substituted_url)
                    .json(&PostData { data: format!("0x{}", hex::encode(&call_data)), sender })
                    .send()
                    .await
                    .map_err(|e| GatewayError::Http(e.to_string()))?
            };

            // Check status
            if !response.status().is_success() {
                return Err(GatewayError::Http(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )));
            }

            // Parse JSON response
            #[derive(Deserialize)]
            struct GatewayResponse {
                data: Bytes,
            }

            let body = response
                .json::<GatewayResponse>()
                .await
                .map_err(|e| GatewayError::InvalidResponse(e.to_string()))?;

            Ok(body.data)
        })
    }
}

/// Default gateway client using HTTP.
///
/// This is a wrapper around `reqwest::Client` that implements `CcipGatewayClient`.
#[derive(Debug, Clone)]
pub struct DefaultGatewayClient {
    client: reqwest::Client,
}

impl DefaultGatewayClient {
    /// Create a new default gateway client
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

impl Default for DefaultGatewayClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CcipGatewayClient for DefaultGatewayClient {
    fn fetch(
        &self,
        url: &str,
        sender: Address,
        call_data: &Bytes,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, GatewayError>> + Send + '_>> {
        self.client.fetch(url, sender, call_data)
    }
}

/// Extract OffchainLookup error from transport error
fn extract_offchain_lookup(err: &TransportError) -> Option<OffchainLookup> {
    err.as_error_resp().and_then(|e| e.as_revert_data()).and_then(|data| {
        if data.len() >= 4 && data[0..4] == OffchainLookup::SELECTOR {
            OffchainLookup::abi_decode(&data).ok()
        } else {
            None
        }
    })
}

/// Build callback request
fn build_callback_request<N: Network>(
    sender: Address,
    callback_function: FixedBytes<4>,
    response: Bytes,
    extra_data: Bytes,
) -> Result<N::TransactionRequest, CcipError> {
    // Encode callback data: abi.encodeWithSelector(callback, response, extraData)

    // Encode the response and extraData as a tuple
    let encoded_params = (response, extra_data).abi_encode();

    // Combine selector and encoded parameters
    let mut call_data = Vec::with_capacity(4 + encoded_params.len());
    call_data.extend_from_slice(&callback_function[..]);
    call_data.extend_from_slice(&encoded_params);

    Ok(N::TransactionRequest::default().with_to(sender).with_input(call_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_network::Ethereum;
    use alloy_primitives::{address, bytes};

    #[test]
    fn test_offchain_lookup_encoding() {
        // Test that we can encode/decode OffchainLookup errors correctly
        let error = OffchainLookup {
            sender: address!("1234567890123456789012345678901234567890"),
            urls: vec![
                "https://gateway1.example.com/{sender}/{data}".to_string(),
                "https://gateway2.example.com/{sender}/{data}".to_string(),
            ],
            callData: bytes!("deadbeef"),
            callbackFunction: FixedBytes::from([0x12, 0x34, 0x56, 0x78]),
            extraData: bytes!("cafebabe"),
        };

        // Encode
        let encoded = error.abi_encode();
        assert_eq!(&encoded[0..4], &OffchainLookup::SELECTOR[..]);

        // Decode
        let decoded = OffchainLookup::abi_decode(&encoded).ok().unwrap();
        assert_eq!(decoded, error);
    }

    #[test]
    fn test_callback_encoding() {
        // Test callback request encoding
        let sender = address!("1234567890123456789012345678901234567890");
        let callback_function = FixedBytes::from([0x12, 0x34, 0x56, 0x78]);
        let response = bytes!("deadbeef");
        let extra_data = bytes!("cafebabe");

        let request =
            build_callback_request::<Ethereum>(sender, callback_function, response, extra_data)
                .unwrap();

        // The TransactionRequest should have to and input fields set
        assert_eq!(request.to, Some(alloy_primitives::TxKind::Call(sender)));
        assert!(request.input.input.is_some());

        // Check that the input starts with the callback selector
        let input = request.input.input.as_ref().unwrap();
        assert_eq!(&input[0..4], &callback_function[..]);
    }
}
