use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use std::{future::Future, pin::Pin, task};
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};
use url::Url;

/// A [reqwest] client that can be used with tower layers.
#[derive(Debug, Clone)]
pub struct LayerClient<S> {
    url: Url,
    service: S,
}

impl<S> LayerClient<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response, Error = reqwest::Error>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    /// Create a new [LayerClient] with the given URL.
    pub const fn new(url: Url, service: S) -> Self {
        Self { url, service }
    }

    /// Make a request using the tower service with layers.
    pub fn request(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("LayerClient", url = %self.url);
        Box::pin(
            async move {
                let mut service = this.service.clone();

                let raw_req = reqwest::Client::new()
                    .post(this.url.to_owned())
                    .json(&req)
                    .build()
                    .map_err(TransportErrorKind::custom)?;

                let resp = service.call(raw_req).await.map_err(TransportErrorKind::custom)?;

                let status = resp.status();

                debug!(%status, "received response from server");

                let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;

                debug!(bytes = body.len(), "retrieved response body. Use `trace` for full body");
                trace!(body = %String::from_utf8_lossy(&body), "response body");

                if status != reqwest::StatusCode::OK {
                    return Err(TransportErrorKind::http_error(
                        status.as_u16(),
                        String::from_utf8_lossy(&body).into_owned(),
                    ));
                }

                serde_json::from_slice(&body)
                    .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))
            }
            .instrument(span),
        )
    }
}

impl<S> TransportConnect for LayerClient<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response, Error = reqwest::Error>
        + Clone
        + Send
        + 'static
        + Sync,
    S::Future: Send,
{
    type Transport = Self;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, TransportError> {
        Box::pin(async move { Ok(Self::new(self.url.clone(), self.service.clone())) })
    }
}

impl<S> Service<RequestPacket> for LayerClient<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response, Error = reqwest::Error>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request(req)
    }
}

/// Future for reqwest responses.
pub type ReqwestResponseFut<T = reqwest::Response, E = reqwest::Error> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;
