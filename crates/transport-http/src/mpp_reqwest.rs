use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportErrorKind, TransportFut, TransportResult};
use itertools::Itertools;
use mpp::{client::PaymentProvider, format_authorization, PaymentChallenge};
use std::task;
use tower::Service;
use tracing::{debug, debug_span, instrument, trace, Instrument};
use url::Url;

/// A reqwest-based HTTP client that automatically handles MPP 402 payment
/// challenges.
///
/// This wraps a [`reqwest::Client`] together with a [`PaymentProvider`].
/// When a request receives a `402 Payment Required` response with a
/// `WWW-Authenticate: Payment ...` header, the client:
/// 1. Parses the [`PaymentChallenge`]
/// 2. Calls [`PaymentProvider::pay`] to obtain a credential
/// 3. Retries the request with an `Authorization: Payment ...` header
///
/// # Example
///
/// ```ignore
/// use alloy_transport_http::{Http, MppReqwestClient};
///
/// let client = Http::mpp_reqwest("https://rpc.example.com".parse()?, provider);
/// ```
#[derive(Clone, Debug)]
pub struct MppReqwestClient<P> {
    client: reqwest::Client,
    provider: P,
}

impl<P> MppReqwestClient<P> {
    /// Create a new [`MppReqwestClient`] with the default reqwest client.
    pub fn new(provider: P) -> Self {
        Self { client: reqwest::Client::new(), provider }
    }

    /// Create a new [`MppReqwestClient`] with a custom reqwest client.
    pub const fn with_client(client: reqwest::Client, provider: P) -> Self {
        Self { client, provider }
    }
}

#[cfg(feature = "mpp-tempo")]
impl MppReqwestClient<mpp::client::TempoProvider> {
    /// Create an [`MppReqwestClient`] backed by a `TempoProvider` in direct
    /// signing mode.
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

    /// Create an [`MppReqwestClient`] backed by a `TempoProvider` in keychain
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

/// An [`Http`] transport using [`reqwest`] with automatic MPP payment handling.
pub type MppReqwestTransport<P> = Http<MppReqwestClient<P>>;

/// Connection details for an [`MppReqwestTransport`].
pub type MppReqwestConnect<P> = HttpConnect<MppReqwestTransport<P>>;

impl<P: PaymentProvider + 'static> Http<MppReqwestClient<P>> {
    /// Create a new [`Http`] transport with MPP payment support.
    pub fn mpp_reqwest(url: Url, provider: P) -> Self {
        Self::with_client(MppReqwestClient::new(provider), url)
    }

    #[instrument(name = "request", skip_all, fields(method_names = %req.method_names().take(3).format(", ").to_string()))]
    async fn do_mpp_reqwest(self, req: RequestPacket) -> TransportResult<ResponsePacket> {
        let resp = self
            .client
            .client
            .post(self.url.clone())
            .json(&req)
            .headers(req.headers())
            .send()
            .await
            .map_err(TransportErrorKind::custom)?;

        let status = resp.status();
        debug!(%status, "received response from server");

        if status == reqwest::StatusCode::PAYMENT_REQUIRED {
            debug!("received 402, attempting MPP payment");

            let www_auth = resp
                .headers()
                .get(reqwest::header::WWW_AUTHENTICATE)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    TransportErrorKind::custom_str("402 response missing WWW-Authenticate header")
                })?
                .to_owned();

            let challenge = PaymentChallenge::from_header(&www_auth).map_err(|e| {
                TransportErrorKind::custom_str(&format!("failed to parse MPP challenge: {e}"))
            })?;

            let credential =
                self.client.provider.pay(&challenge).await.map_err(|e| {
                    TransportErrorKind::custom_str(&format!("MPP payment failed: {e}"))
                })?;

            let auth_value = format_authorization(&credential).map_err(|e| {
                TransportErrorKind::custom_str(&format!("failed to format MPP authorization: {e}"))
            })?;

            debug!("MPP payment succeeded, retrying request");

            let retry_resp = self
                .client
                .client
                .post(self.url)
                .json(&req)
                .headers(req.headers())
                .header(reqwest::header::AUTHORIZATION, &auth_value)
                .send()
                .await
                .map_err(TransportErrorKind::custom)?;

            let retry_status = retry_resp.status();
            debug!(%retry_status, "received retry response from server");

            let body = retry_resp.bytes().await.map_err(TransportErrorKind::custom)?;

            if tracing::enabled!(tracing::Level::TRACE) {
                trace!(body = %String::from_utf8_lossy(&body), "response body");
            } else {
                debug!(bytes = body.len(), "retrieved response body");
            }

            if !retry_status.is_success() {
                return Err(TransportErrorKind::http_error(
                    retry_status.as_u16(),
                    String::from_utf8_lossy(&body).into_owned(),
                ));
            }

            return serde_json::from_slice(&body)
                .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)));
        }

        let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;

        if tracing::enabled!(tracing::Level::TRACE) {
            trace!(body = %String::from_utf8_lossy(&body), "response body");
        } else {
            debug!(bytes = body.len(), "retrieved response body");
        }

        if !status.is_success() {
            return Err(TransportErrorKind::http_error(
                status.as_u16(),
                String::from_utf8_lossy(&body).into_owned(),
            ));
        }

        serde_json::from_slice(&body)
            .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))
    }
}

impl<P: PaymentProvider + 'static> Service<RequestPacket> for Http<MppReqwestClient<P>> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("MppReqwestTransport", url = %this.url);
        Box::pin(this.do_mpp_reqwest(req).instrument(span.or_current()))
    }
}
