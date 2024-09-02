use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use std::task;
use tower::{Service, ServiceBuilder};
use tracing::{debug, debug_span, trace, Instrument};
use url::Url;

/// A [reqwest] client that can be used with tower layers.
#[derive(Debug, Clone)]
pub struct LayerClient {
    url: Url,
}

impl LayerClient {
    /// Create a new [LayerClient] with the given URL.
    pub const fn new(url: Url) -> Self {
        Self { url }
    }

    /// Make a request using the tower service with layers.
    pub fn request(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("LayerClient", url = %self.url);
        Box::pin(
            async move {
                let client = reqwest::Client::new();

                let mut service = ServiceBuilder::new().service(client);

                let reqwest_request = service
                    .post(this.url.to_owned())
                    .json(&req)
                    .build()
                    .map_err(TransportErrorKind::custom)?;

                let resp =
                    service.call(reqwest_request).await.map_err(TransportErrorKind::custom)?;

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

impl TransportConnect for LayerClient {
    type Transport = LayerClient;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, TransportError> {
        Box::pin(async move { Ok(LayerClient::new(self.url.clone())) })
    }
}

impl Service<RequestPacket> for LayerClient {
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
