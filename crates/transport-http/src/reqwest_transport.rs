use crate::Http;
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportErrorKind, TransportFut};
use std::task;
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};
use url::Url;

/// Rexported from [`reqwest`](::reqwest).
pub use reqwest::Client;

/// An [`Http`] transport using [`reqwest`](::reqwest).
pub type ReqwestTransport = Http<Client>;

impl Http<Client> {
    /// Create a new [`Http`] transport.
    pub fn new(url: Url) -> Self {
        Self { client: Default::default(), url }
    }

    /// Make a request.
    fn request_reqwest(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span: tracing::Span = debug_span!("ReqwestTransport", url = %self.url);
        Box::pin(
            async move {
                let resp = this
                    .client
                    .post(this.url)
                    .json(&req)
                    .send()
                    .await
                    .map_err(TransportErrorKind::custom)?;
                let status = resp.status();

                debug!(%status, "received response from server");

                // Unpack data from the response body. We do this regardless of
                // the status code, as we want to return the error in the body
                // if there is one.
                let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;

                debug!(bytes = body.len(), "retrieved response body. Use `trace` for full body");
                trace!(body = %String::from_utf8_lossy(&body), "response body");

                if status != reqwest::StatusCode::OK {
                    return Err(TransportErrorKind::custom_str(&format!(
                        "HTTP error {status} with body: {}",
                        String::from_utf8_lossy(&body)
                    )));
                }

                // Deser a Box<RawValue> from the body. If deser fails, return
                // the body as a string in the error. The conversion to String
                // is lossy and may not cover all the bytes in the body.
                serde_json::from_slice(&body)
                    .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))
            }
            .instrument(span),
        )
    }
}

impl Service<RequestPacket> for Http<reqwest::Client> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // reqwest always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_reqwest(req)
    }
}

impl Service<RequestPacket> for &Http<reqwest::Client> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // reqwest always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_reqwest(req)
    }
}
