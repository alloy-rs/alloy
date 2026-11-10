use crate::{body::BodyError, Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, BoxTransport, TransportConnect, TransportError, TransportErrorKind,
    TransportFut, TransportResult,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Body, Buf, Bytes, Incoming},
    header, Request, Response,
};
use hyper_util::client::legacy::Error;
use itertools::Itertools;
use std::{future::Future, marker::PhantomData, pin::Pin, task, time::Duration};
use tower::{Layer, Service};
use tracing::{debug, debug_span, instrument, Instrument};

type Hyper = hyper_util::client::legacy::Client<
    hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<::hyper::body::Bytes>,
>;

/// A [`hyper`] based transport client.
pub type HyperTransport = Http<HyperClient>;

impl HyperTransport {
    /// Create a new [`HyperTransport`] with the given URL and default hyper client.
    ///
    /// See [`HyperClient::new`] for the client's default timeouts.
    pub fn new_hyper(url: url::Url) -> Self {
        let client = HyperClient::new();
        Self::with_client(client, url)
    }
}

/// A [hyper] based client that can be used with tower layers.
#[derive(Clone, Debug)]
pub struct HyperClient<B = Full<Bytes>, S = Hyper> {
    service: S,
    read_timeout: Option<Duration>,
    _pd: PhantomData<B>,
}

/// Alias for [`Response<Incoming>`]
pub type HyperResponse = Response<Incoming>;

/// Alias for pinned box future that results in [`HyperResponse`]
pub type HyperResponseFut<T = HyperResponse, E = Error> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

impl HyperClient {
    /// Create a new [HyperClient] with the default hyper client.
    ///
    /// The client has a [`DEFAULT_CONNECT_TIMEOUT`](crate::DEFAULT_CONNECT_TIMEOUT) and a
    /// [`DEFAULT_READ_TIMEOUT`](crate::DEFAULT_READ_TIMEOUT). Use
    /// [`HyperClient::with_read_timeout`] to change the read timeout, or
    /// [`HyperClient::with_service`] to provide a client with other timeouts.
    ///
    /// # Panics
    ///
    /// Panics if the TLS connector cannot be created, like [`hyper_tls::HttpsConnector::new`].
    pub fn new() -> Self {
        let mut http = hyper_util::client::legacy::connect::HttpConnector::new();
        http.enforce_http(false);
        http.set_connect_timeout(Some(crate::DEFAULT_CONNECT_TIMEOUT));
        let executor = hyper_util::rt::TokioExecutor::new();
        let service = hyper_util::client::legacy::Client::builder(executor)
            .build(hyper_tls::HttpsConnector::new_with_connector(http));
        Self::with_service(service).with_read_timeout(Some(crate::DEFAULT_READ_TIMEOUT))
    }
}

impl Default for HyperClient {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S> HyperClient<B, S> {
    /// Create a new [HyperClient] with the given service.
    ///
    /// No read timeout is added to the service.
    pub const fn with_service(service: S) -> Self {
        Self { service, read_timeout: None, _pd: PhantomData }
    }

    /// Set the read timeout, or `None` for no read timeout.
    ///
    /// The request fails if the service does not return the response headers within this time,
    /// or if the response body then yields no data for this long. See
    /// [`DEFAULT_READ_TIMEOUT`](crate::DEFAULT_READ_TIMEOUT).
    pub const fn with_read_timeout(mut self, read_timeout: Option<Duration>) -> Self {
        self.read_timeout = read_timeout;
        self
    }

    /// Get the read timeout, if any.
    pub const fn read_timeout(&self) -> Option<Duration> {
        self.read_timeout
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
        HyperClient::with_service(layer.layer(self.service)).with_read_timeout(self.read_timeout)
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

        let request_timeout = self.settings.request_timeout();
        let start = tokio::time::Instant::now();
        let read_timeout = self.client.read_timeout;
        let mut service = self.client.service;
        let resp = timeout(request_timeout, timeout(read_timeout, service.call(req)))
            .await
            .map_err(|duration| {
                let msg = format!("HTTP request timed out after {duration:?}");
                TransportErrorKind::custom_str(&msg)
            })?
            .map_err(|duration| {
                let msg = format!("no HTTP response received within {duration:?}");
                TransportErrorKind::custom_str(&msg)
            })?
            .map_err(TransportErrorKind::custom)?;

        let status = resp.status();
        let retry_after = crate::parse_retry_after(
            resp.headers().get(header::RETRY_AFTER).and_then(|value| value.to_str().ok()),
        );

        debug!(%status, "received response from server");

        let max = self.settings.max_response_size();
        let collect = collect_body(resp.into_body(), max, read_timeout);
        // The rest of the deadline bounds the body on its own, so that an error response that
        // times out keeps its status and `Retry-After`.
        let body = match request_timeout {
            Some(request_timeout) => {
                let remaining = request_timeout.saturating_sub(start.elapsed());
                tokio::time::timeout(remaining, collect)
                    .await
                    .unwrap_or(Err(BodyError::RequestTimedOut(request_timeout)))
            }
            None => collect.await,
        };
        crate::handle_response(status.as_u16(), retry_after, body)
    }
}

/// Awaits `fut`, failing with the elapsed `duration` if it does not complete in time, if set.
async fn timeout<F: Future>(duration: Option<Duration>, fut: F) -> Result<F::Output, Duration> {
    match duration {
        Some(duration) => tokio::time::timeout(duration, fut).await.map_err(|_| duration),
        None => Ok(fut.await),
    }
}

/// Collects the response body, failing as soon as it exceeds `max` bytes, or when no data arrives
/// within `read_timeout`, if set.
async fn collect_body<B: Body>(
    body: B,
    max: Option<usize>,
    read_timeout: Option<Duration>,
) -> Result<Bytes, BodyError<B::Error>> {
    if let Some(max) = max.filter(|&max| body.size_hint().lower() > max as u64) {
        return Err(BodyError::TooLarge(max));
    }

    let mut body = std::pin::pin!(body);
    let mut buf = Vec::new();
    while let Some(frame) =
        timeout(read_timeout, body.frame()).await.map_err(BodyError::TimedOut)?
    {
        let Ok(mut data) = frame.map_err(BodyError::Read)?.into_data() else { continue };
        if let Some(max) = max.filter(|&max| buf.len() + data.remaining() > max) {
            return Err(BodyError::TooLarge(max));
        }
        while data.has_remaining() {
            let chunk = data.chunk();
            let len = chunk.len();
            buf.extend_from_slice(chunk);
            data.advance(len);
        }
    }
    Ok(buf.into())
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
        let span = debug_span!(
            "HyperTransport",
            scheme = this.url.scheme(),
            host = this.url.host_str().unwrap_or_default(),
            port = this.url.port_or_known_default(),
        );
        Box::pin(this.do_hyper(req).instrument(span.or_current()))
    }
}
