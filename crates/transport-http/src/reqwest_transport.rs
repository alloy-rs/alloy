use crate::{body::BodyError, Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, BoxTransport, TransportConnect, TransportError, TransportErrorKind,
    TransportFut, TransportResult,
};
use itertools::Itertools;
use std::task;
use tower::Service;
use tracing::{debug, debug_span, instrument, Instrument};
use url::Url;

/// Rexported from [`reqwest`].
pub use reqwest::Client;

/// An [`Http`] transport using [`reqwest`].
pub type ReqwestTransport = Http<Client>;

/// Connection details for a [`ReqwestTransport`].
pub type ReqwestConnect = HttpConnect<ReqwestTransport>;

impl TransportConnect for ReqwestConnect {
    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    async fn get_transport(&self) -> Result<BoxTransport, TransportError> {
        Ok(BoxTransport::new(Http::with_client(Client::new(), self.url.clone())))
    }
}

impl Http<Client> {
    /// Create a new [`Http`] transport.
    pub fn new(url: Url) -> Self {
        Self::with_client(Default::default(), url)
    }

    #[instrument(name = "request", skip_all, fields(method_names = %req.method_names().take(3).format(", ").to_string()))]
    async fn do_reqwest(self, req: RequestPacket) -> TransportResult<ResponsePacket> {
        let resp = self
            .client
            .post(self.url)
            .json(&req)
            .headers(req.headers())
            .send()
            .await
            .map_err(TransportErrorKind::custom)?;
        let status = resp.status();
        let retry_after = crate::parse_retry_after(
            resp.headers().get(reqwest::header::RETRY_AFTER).and_then(|value| value.to_str().ok()),
        );

        debug!(%status, "received response from server");

        let body = match self.settings.max_response_size() {
            None => resp.bytes().await.map_err(BodyError::Read),
            Some(max) => read_body_limited(resp, max).await.map(Into::into),
        };
        crate::handle_response(status.as_u16(), retry_after, body)
    }
}

/// Reads the response body, failing as soon as it exceeds `max` bytes.
async fn read_body_limited(
    resp: reqwest::Response,
    max: usize,
) -> Result<Vec<u8>, BodyError<reqwest::Error>> {
    if resp.content_length().is_some_and(|len| len > max as u64) {
        return Err(BodyError::TooLarge(max));
    }

    // wasm responses can't be read incrementally, so the size is only checked after reading.
    #[cfg(target_family = "wasm")]
    {
        let body = resp.bytes().await.map_err(BodyError::Read)?;
        if body.len() > max {
            return Err(BodyError::TooLarge(max));
        }
        Ok(body.into())
    }

    #[cfg(not(target_family = "wasm"))]
    {
        let mut resp = resp;
        let mut body = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(BodyError::Read)? {
            if body.len() + chunk.len() > max {
                return Err(BodyError::TooLarge(max));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

impl Service<RequestPacket> for Http<reqwest::Client> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // `reqwest` always returns `Ok(())`.
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        let span = debug_span!(
            "ReqwestTransport",
            scheme = this.url.scheme(),
            host = this.url.host_str().unwrap_or_default(),
            port = this.url.port_or_known_default(),
        );
        Box::pin(this.do_reqwest(req).instrument(span.or_current()))
    }
}
