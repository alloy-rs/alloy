use crate::hyper::{header, Request, Response};
use alloy_transport::{TransportError, TransportErrorKind};
use hyper::header::HeaderValue;
use mpp::{client::PaymentProvider, format_authorization, PaymentChallenge};
use std::{future::Future, pin::Pin, task};
use tower::{Layer, Service};
use tracing::debug;

/// A tower [`Layer`] that intercepts HTTP 402 responses and automatically
/// handles payment challenges using the [Machine Payments Protocol (MPP)].
///
/// When the upstream service returns a `402 Payment Required` response with
/// a `WWW-Authenticate: Payment ...` header, this layer:
/// 1. Parses the [`PaymentChallenge`] from the header
/// 2. Calls [`PaymentProvider::pay`] to obtain a [`PaymentCredential`]
/// 3. Retries the original request with an `Authorization: Payment ...` header
///
/// Non-402 responses pass through unchanged.
///
/// # Example
///
/// ```ignore
/// use alloy_transport_http::{HyperClient, MppLayer};
///
/// let client = HyperClient::new()
///     .layer(MppLayer::new(my_provider));
/// ```
///
/// [Machine Payments Protocol (MPP)]: https://github.com/tempoxyz/mpp-rs
/// [`PaymentCredential`]: mpp::PaymentCredential
#[derive(Clone, Debug)]
pub struct MppLayer<P> {
    provider: P,
}

impl<P> MppLayer<P> {
    /// Create a new [`MppLayer`] with the given [`PaymentProvider`].
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

#[cfg(feature = "mpp-tempo")]
impl MppLayer<mpp::client::TempoProvider> {
    /// Create an [`MppLayer`] backed by a `TempoProvider` in direct signing mode.
    ///
    /// # Errors
    ///
    /// Returns an error if `rpc_url` is not a valid URL.
    pub fn with_tempo_signer(
        signer: mpp::PrivateKeySigner,
        rpc_url: impl AsRef<str>,
    ) -> Result<Self, mpp::MppError> {
        Ok(Self::new(mpp::client::TempoProvider::new(signer, rpc_url)?))
    }

    /// Create an [`MppLayer`] backed by a `TempoProvider` in keychain
    /// (access key) signing mode.
    ///
    /// # Errors
    ///
    /// Returns an error if `rpc_url` is not a valid URL.
    pub fn with_tempo_access_key(
        signer: mpp::PrivateKeySigner,
        wallet_address: mpp::Address,
        rpc_url: impl AsRef<str>,
    ) -> Result<Self, mpp::MppError> {
        use mpp::client::tempo::signing::{KeychainVersion, TempoSigningMode};

        Ok(Self::new(mpp::client::TempoProvider::new(signer, rpc_url)?.with_signing_mode(
            TempoSigningMode::Keychain {
                wallet: wallet_address,
                key_authorization: None,
                version: KeychainVersion::V2,
            },
        )))
    }
}

impl<S, P: Clone> Layer<S> for MppLayer<P> {
    type Service = MppService<S, P>;

    fn layer(&self, inner: S) -> Self::Service {
        MppService { inner, provider: self.provider.clone() }
    }
}

/// A service that handles MPP 402 payment challenges automatically.
///
/// See [`MppLayer`] for details.
#[derive(Clone, Debug)]
pub struct MppService<S, P> {
    inner: S,
    provider: P,
}

impl<S, B, ResBody, P> Service<Request<B>> for MppService<S, P>
where
    S: Service<Request<B>, Response = Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: std::error::Error + Send + Sync + 'static,
    ResBody::Data: Send,
    P: PaymentProvider + 'static,
{
    type Response = Response<ResBody>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Response<ResBody>, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(TransportErrorKind::custom)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut service = self.inner.clone();
        let provider = self.provider.clone();
        let body = req.body().clone();
        let original_parts = req.uri().clone();
        let original_headers = req.headers().clone();
        let original_method = req.method().clone();

        Box::pin(async move {
            let resp = service.call(req).await.map_err(TransportErrorKind::custom)?;

            if resp.status() != hyper::StatusCode::PAYMENT_REQUIRED {
                return Ok(resp);
            }

            debug!("received 402, attempting MPP payment");

            let www_auth = resp
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    TransportErrorKind::custom_str("402 response missing WWW-Authenticate header")
                })?
                .to_owned();

            let challenge = PaymentChallenge::from_header(&www_auth).map_err(|e| {
                TransportErrorKind::custom_str(&format!("failed to parse MPP challenge: {e}"))
            })?;

            let credential = provider
                .pay(&challenge)
                .await
                .map_err(|e| TransportErrorKind::custom_str(&format!("MPP payment failed: {e}")))?;

            let auth_value = format_authorization(&credential).map_err(|e| {
                TransportErrorKind::custom_str(&format!("failed to format MPP authorization: {e}"))
            })?;

            debug!("MPP payment succeeded, retrying request");

            let mut retry = Request::builder().method(original_method).uri(original_parts);

            for (name, value) in original_headers.iter() {
                retry = retry.header(name, value);
            }

            let retry = retry
                .header(
                    header::AUTHORIZATION,
                    HeaderValue::from_str(&auth_value).map_err(TransportErrorKind::custom)?,
                )
                .body(body)
                .map_err(TransportErrorKind::custom)?;

            service.call(retry).await.map_err(TransportErrorKind::custom)
        })
    }
}
