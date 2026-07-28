use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, BoxTransport, TransportConnect, TransportError, TransportErrorKind,
    TransportFut, TransportResult,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    header, Request, Response,
};
use hyper_util::client::legacy::Error;
use itertools::Itertools;
use std::{future::Future, marker::PhantomData, pin::Pin, task};
use tower::{Layer, Service};
use tracing::{debug, debug_span, instrument, trace, Instrument};

#[cfg(feature = "hyper-tls")]
type Hyper = hyper_util::client::legacy::Client<
    hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<::hyper::body::Bytes>,
>;

#[cfg(not(feature = "hyper-tls"))]
type Hyper = hyper_util::client::legacy::Client<
    hyper_util::client::legacy::connect::HttpConnector,
    http_body_util::Full<::hyper::body::Bytes>,
>;

/// A [`hyper`] based transport client.
pub type HyperTransport = Http<HyperClient>;

impl HyperTransport {
    /// Create a new [`HyperTransport`] with the given URL and default hyper client.
    pub fn new_hyper(url: url::Url) -> Self {
        let client = HyperClient::new();
        Self::with_client(client, url)
    }
}

/// A [hyper] based client that can be used with tower layers.
#[derive(Clone, Debug)]
pub struct HyperClient<B = Full<Bytes>, S = Hyper> {
    service: S,
    _pd: PhantomData<B>,
}

/// Alias for [`Response<Incoming>`]
pub type HyperResponse = Response<Incoming>;

/// Alias for pinned box future that results in [`HyperResponse`]
pub type HyperResponseFut<T = HyperResponse, E = Error> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

impl HyperClient {
    /// Create a new [HyperClient] with the given URL and default hyper client.
    pub fn new() -> Self {
        let executor = hyper_util::rt::TokioExecutor::new();

        #[cfg(feature = "hyper-tls")]
        let service = hyper_util::client::legacy::Client::builder(executor)
            .build(hyper_tls::HttpsConnector::new());

        #[cfg(not(feature = "hyper-tls"))]
        let service =
            hyper_util::client::legacy::Client::builder(executor).build_http::<Full<Bytes>>();
        Self { service, _pd: PhantomData }
    }
}

impl Default for HyperClient {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S> HyperClient<B, S> {
    /// Create a new [HyperClient] with the given URL and service.
    pub const fn with_service(service: S) -> Self {
        Self { service, _pd: PhantomData }
    }

    /// Apply a tower [`Layer`] to this client's service.
    ///
    /// This allows you to compose middleware layers following the tower pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #use alloy_transport_http::HyperClient;
    /// #use alloy_transport_http::AuthLayer;
    /// #use alloy_rpc_types_engine::JwtSecret;
    ///
    /// let secret = JwtSecret::random();
    /// let client = HyperClient::new()
    ///     .layer(AuthLayer::new(secret));
    /// ```
    pub fn layer<L>(self, layer: L) -> HyperClient<B, L::Service>
    where
        L: Layer<S>,
    {
        HyperClient::with_service(layer.layer(self.service))
    }
}

impl<B, S, ResBody> Http<HyperClient<B, S>>
where
    S: Service<Request<B>, Response = Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone,
    ResBody: BodyExt + Send + 'static,
    ResBody::Error: std::error::Error + Send + Sync + 'static,
    ResBody::Data: Send,
{
    #[instrument(name = "request", skip_all, fields(method_names = %req.method_names().take(3).format(", ").to_string()))]
    async fn do_hyper(self, req: RequestPacket) -> TransportResult<ResponsePacket> {
        debug!(count = req.len(), "sending request packet to server");

        let mut builder = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(self.url.as_str())
            .header(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));

        // Add any additional headers from the request packet.
        for (name, value) in req.headers().iter() {
            builder = builder.header(name, value);
        }

        let ser = req.serialize().map_err(TransportError::ser_err)?;
        // convert the Box<RawValue> into a hyper request<B>
        let body = ser.get().as_bytes().to_owned().into();

        let req = builder.body(body).map_err(TransportErrorKind::custom)?;

        let mut service = self.client.service;
        let resp = service.call(req).await.map_err(TransportErrorKind::custom)?;

        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(crate::parse_retry_after);

        debug!(%status, "received response from server");

        // Unpack data from the response body. We do this regardless of
        // the status code, as we want to return the error in the body
        // if there is one.
        let body = resp.into_body().collect().await.map_err(TransportErrorKind::custom)?.to_bytes();

        if tracing::enabled!(tracing::Level::TRACE) {
            trace!(body = %String::from_utf8_lossy(&body), "response body");
        } else {
            debug!(bytes = body.len(), "retrieved response body");
        }

        if !status.is_success() {
            let body = String::from_utf8_lossy(&body).into_owned();
            if let Some(retry_after) = retry_after {
                return Err(TransportErrorKind::http_error_with_retry_after(
                    status.as_u16(),
                    body,
                    retry_after,
                ));
            }

            if let Some(response) = crate::json_rpc_error_response(body.as_bytes()) {
                return Ok(response);
            }

            return Err(TransportErrorKind::http_error(status.as_u16(), body));
        }

        // Deserialize a Box<RawValue> from the body. If deserialization fails, return
        // the body as a string in the error. The conversion to String
        // is lossy and may not cover all the bytes in the body.
        serde_json::from_slice(&body)
            .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(body.as_ref())))
    }
}

impl TransportConnect for HttpConnect<HyperTransport> {
    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    async fn get_transport(&self) -> Result<BoxTransport, TransportError> {
        Ok(BoxTransport::new(Http::with_client(HyperClient::new(), self.url.clone())))
    }
}

impl<B, S> Service<RequestPacket> for Http<HyperClient<B, S>>
where
    S: Service<Request<B>, Response = HyperResponse> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // `hyper` always returns `Ok(())`.
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("HyperTransport", url = %this.url);
        Box::pin(this.do_hyper(req).instrument(span.or_current()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request as RpcRequest};
    use std::{convert::Infallible, time::Duration};

    const BODY: &str =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":429,"message":"Rate Limit Hit"}}"#;

    fn request() -> RequestPacket {
        RequestPacket::Single(
            RpcRequest::new("eth_chainId", Id::Number(1), ()).serialize().unwrap(),
        )
    }

    #[tokio::test]
    async fn preserves_retry_after_from_http_error() {
        let service = tower::service_fn(|_: Request<Full<Bytes>>| async {
            Ok::<_, Infallible>(
                Response::builder()
                    .status(429)
                    .header(header::RETRY_AFTER, "52")
                    .body(Full::new(Bytes::from_static(BODY.as_bytes())))
                    .unwrap(),
            )
        });
        let client = HyperClient::with_service(service);
        let transport = Http::with_client(client, "http://localhost".parse().unwrap());
        let error = transport.do_hyper(request()).await.unwrap_err();

        let TransportError::Transport(error) = error else { panic!("expected transport error") };
        assert_eq!(error.retry_after(), Some(Duration::from_secs(52)));
        assert_eq!(error.as_http_error().unwrap().status, 429);
        assert_eq!(error.as_http_error().unwrap().body, BODY);
    }
}
