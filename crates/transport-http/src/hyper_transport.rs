use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    header, Request, Response,
};
use hyper_util::client::legacy::Error;
use std::{future::Future, marker::PhantomData, pin::Pin, task};
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};

use crate::{Http, HttpConnect};

/// A [`hyper`] HTTP client.
pub type HyperClient = hyper_util::client::legacy::Client<
    hyper_util::client::legacy::connect::HttpConnector,
    http_body_util::Full<::hyper::body::Bytes>,
>;

/// A [hyper] client that can be used with tower layers.
#[derive(Clone, Debug)]
pub struct HyperTransport<B = Full<Bytes>, S = HyperClient> {
    service: S,
    _pd: PhantomData<B>,
}

/// Alias for [`Response<Incoming>`]
pub type HyperResponse = Response<Incoming>;

/// Alias for pinned box future that results in [`HyperResponse`]
pub type HyperResponseFut<T = HyperResponse, E = Error> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

impl HyperTransport {
    /// Create a new [HyperTransport] with the given URL and default hyper client.
    pub fn new() -> Self {
        let executor = hyper_util::rt::TokioExecutor::new();

        let service =
            hyper_util::client::legacy::Client::builder(executor).build_http::<Full<Bytes>>();

        Self { service, _pd: PhantomData }
    }
}

impl Default for HyperTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S> HyperTransport<B, S> {
    /// Create a new [HyperTransport] with the given URL and service.
    pub const fn with_service(service: S) -> Self {
        Self { service, _pd: PhantomData }
    }
}

impl<B, S> Http<HyperTransport<B, S>>
where
    S: Service<Request<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone,
{
    /// Make a request to the server using the given service.
    fn request_hyper(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("HyperTransport", url = %this.url);
        Box::pin(
            async move {
                debug!(count = req.len(), "sending request packet to server");
                let ser = req.serialize().map_err(TransportError::ser_err)?;
                // convert the Box<RawValue> into a hyper request<B>
                let body = ser.get().as_bytes().to_owned().into();

                let req = hyper::Request::builder()
                    .method(hyper::Method::POST)
                    .uri(this.url.as_str())
                    .header(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_static("application/json"),
                    )
                    .body(body)
                    .expect("request parts are invalid");

                let mut service = this.client.service.clone();
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

impl TransportConnect for HttpConnect<HyperTransport> {
    type Transport = Http<HyperTransport>;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, TransportError> {
        Box::pin(async move {
            let hyper_t = HyperTransport::new();

            Ok(Http::with_client(hyper_t, self.url.clone()))
        })
    }
}

impl<B, S> Service<RequestPacket> for Http<HyperTransport<B, S>>
where
    S: Service<Request<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_hyper(req)
    }
}

impl<B, S> Service<RequestPacket> for &Http<HyperTransport<B, S>>
where
    S: Service<Request<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_hyper(req)
    }
}
