use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Buf, Bytes, Incoming},
    header, Request, Response,
};
use hyper_util::client::legacy::Error;
use std::{future::Future, marker::PhantomData, pin::Pin, task};
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};
use url::Url;

/// A [hyper] client that can be used with tower layers.
#[derive(Clone, Debug)]
pub struct HyperLayerTransport<S, B> {
    url: Url,
    service: S,
    _pd: PhantomData<B>,
}

/// Alias for [`Request<Full<B>>`]
pub type HyperRequest<B> = Request<Full<B>>;

/// Alias for [`Response<Incoming>`]
pub type HyperResponse = Response<Incoming>;

/// Alias for pinned box future that results in [`HyperResponse`]
pub type HyperResponseFut<T = HyperResponse, E = Error> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

impl<S, B> HyperLayerTransport<S, B>
where
    S: Service<HyperRequest<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static + Clone,
{
    /// Create a new [HyperLayerTransport] with the given URL and service.
    pub const fn new(url: Url, service: S) -> Self {
        Self { url, service, _pd: PhantomData }
    }

    /// Make a request to the server using the given service.
    pub fn request(&mut self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("HyperLayerTransport", url = %this.url);
        Box::pin(
            async move {
                debug!(count = req.len(), "sending request packet to server");
                let ser = req.serialize().map_err(TransportError::ser_err)?;
                // convert the Box<RawValue> into a hyper request<B>
                let body = Full::from(Bytes::from(<Box<[u8]>>::from(<Box<str>>::from(ser))));

                let req = hyper::Request::builder()
                    .method(hyper::Method::POST)
                    .uri(this.url.as_str())
                    .header(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_static("application/json"),
                    )
                    .body(body)
                    .expect("request parts are invalid");

                let mut service = this.service.clone();
                let resp = service.call(req).await.map_err(TransportErrorKind::custom)?;

                let status = resp.status();

                debug!(%status, "received response from server");

                // Unpack data from the response body. We do this regardless of
                // the status code, as we want to return the error in the body
                // if there is one.
                let body = resp
                    .into_body()
                    .collect()
                    .await
                    .map_err(TransportErrorKind::custom)?
                    .to_bytes();

                debug!(bytes = body.len(), "retrieved response body. Use `trace` for full body");
                trace!(body = %String::from_utf8_lossy(&body), "response body");

                if status != hyper::StatusCode::OK {
                    return Err(TransportErrorKind::http_error(
                        status.as_u16(),
                        String::from_utf8_lossy(&body).into_owned(),
                    ));
                }

                // Deserialize a Box<RawValue> from the body. If deserialization fails, return
                // the body as a string in the error. The conversion to String
                // is lossy and may not cover all the bytes in the body.
                serde_json::from_slice(&body).map_err(|err| {
                    TransportError::deser_err(err, String::from_utf8_lossy(body.as_ref()))
                })
            }
            .instrument(span),
        )
    }
}

impl<S, B> TransportConnect for HyperLayerTransport<S, B>
where
    S: Service<HyperRequest<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static + Clone + Sync,
{
    type Transport = Self;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, alloy_transport::TransportError> {
        Box::pin(async move { Ok(Self::new(self.url.clone(), self.service.clone())) })
    }
}

impl<S, B> Service<RequestPacket> for HyperLayerTransport<S, B>
where
    S: Service<HyperRequest<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static + Clone + Sync,
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
