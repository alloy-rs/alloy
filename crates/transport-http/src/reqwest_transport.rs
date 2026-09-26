use crate::{body::BodyError, Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, BoxTransport, TransportConnect, TransportError, TransportErrorKind,
    TransportFut, TransportResult,
};
use itertools::Itertools;
use std::{task, time::Duration};
use tower::Service;
use tracing::{debug, debug_span, instrument, trace, Instrument};
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
        // Delay requested by the server via a `Retry-After` header, delay-seconds form only.
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .map(Duration::from_secs);

        debug!(%status, "received response from server");

        // Unpack data from the response body. We do this regardless of
        // the status code, as we want to return the error in the body
        // if there is one.
        let body = match self.settings.max_response_size() {
            None => resp.bytes().await.map_err(BodyError::Read),
            Some(max) => read_body_limited(resp, max).await.map(Into::into),
        };
        let body = match body {
            Ok(body) => body,
            // A failed body read on an error response still carries retryable metadata.
            Err(err) if !status.is_success() => {
                return Err(TransportErrorKind::http_error_with_retry_after(
                    status.as_u16(),
                    format!("<failed to read response body: {err}>"),
                    retry_after,
                ));
            }
            Err(err) => return Err(err.into_transport_error()),
        };

        if tracing::enabled!(tracing::Level::TRACE) {
            trace!(body = %String::from_utf8_lossy(&body), "response body");
        } else {
            debug!(bytes = body.len(), "retrieved response body");
        }

        if !status.is_success() {
            return crate::http_error_response(status.as_u16(), &body, retry_after);
        }

        // Deserialize a Box<RawValue> from the body. If deserialization fails, return
        // the body as a string in the error. The conversion to String
        // is lossy and may not cover all the bytes in the body.
        serde_json::from_slice(&body)
            .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))
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
        let span = debug_span!("ReqwestTransport", url = %this.url);
        Box::pin(this.do_reqwest(req).instrument(span.or_current()))
    }
}
