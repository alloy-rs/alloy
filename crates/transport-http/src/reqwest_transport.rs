use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, Authorization, TransportConnect, TransportError, TransportErrorKind,
    TransportFut,
};
use std::task;
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};
use url::Url;

/// Rexported from [`reqwest`].
pub use reqwest::Client;

/// An [`Http`] transport using [`reqwest`].
pub type ReqwestTransport = Http<Client>;

/// Connection details for a [`ReqwestTransport`].
pub type ReqwestConnect = HttpConnect<ReqwestTransport>;

impl TransportConnect for ReqwestConnect {
    type Transport = ReqwestTransport;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, TransportError> {
        Box::pin(async move { Ok(Http::with_client(Client::new(), self.url.clone())) })
    }
}

impl Http<Client> {
    /// Create a new [`Http`] transport.
    pub fn new(url: Url) -> Self {
        Self { client: Default::default(), url, auth: None }
    }

    /// With authorization header.
    pub fn with_auth(mut self, auth: Option<Authorization>) -> Self {
        self.auth = auth;
        self
    }

    /// Make a request.
    fn request_reqwest(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span: tracing::Span = debug_span!("ReqwestTransport", url = %self.url);
        Box::pin(
            async move {
                let mut builder = this.client.post(this.url);

                if let Some(auth) = this.auth {
                    builder = builder.header(reqwest::header::AUTHORIZATION, auth.to_string());
                }

                let resp = builder.json(&req).send().await.map_err(TransportErrorKind::custom)?;
                let status = resp.status();

                debug!(%status, "received response from server");

                // Unpack data from the response body. We do this regardless of
                // the status code, as we want to return the error in the body
                // if there is one.
                let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;

                debug!(bytes = body.len(), "retrieved response body. Use `trace` for full body");
                trace!(body = %String::from_utf8_lossy(&body), "response body");

                if status != reqwest::StatusCode::OK {
                    return Err(TransportErrorKind::http_error(
                        status.as_u16(),
                        String::from_utf8_lossy(&body).into_owned(),
                    ));
                }

                // Deserialize a Box<RawValue> from the body. If deserialization fails, return
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
