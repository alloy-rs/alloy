use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use std::task;
use tower::{
    layer::util::{Identity, Stack},
    Layer, Service, ServiceBuilder,
};
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
        Self { client: Default::default(), url }
    }

    /// Make a request.
    fn _request_reqwest(&self, req: RequestPacket) -> TransportFut<'static> {
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

    /// Make a request using the tower service with layers.
    fn request_reqwest_with_layers(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("ReqwestTransport", url = %self.url);
        let client = self.client.clone();
        Box::pin(
            async move {
                let mut service =
                    ReqwestBuilder::default().layer(LoggingLayer).on_transport(this.clone());

                let reqwest_request =
                    client.post(this.url).json(&req).build().map_err(TransportErrorKind::custom)?;

                let resp =
                    service.call(reqwest_request).await.map_err(TransportErrorKind::custom)?;

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
        self.request_reqwest_with_layers(req)
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
        self.request_reqwest_with_layers(req)
    }
}

type ReqwestFuture =
    Pin<Box<dyn Future<Output = Result<reqwest::Response, reqwest::Error>> + Send>>;

impl Service<reqwest::Request> for Http<reqwest::Client> {
    type Response = reqwest::Response;
    type Error = reqwest::Error;
    type Future = ReqwestFuture;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: reqwest::Request) -> Self::Future {
        let reqwest_client = self.client.clone();
        let future = reqwest_client.execute(req);
        Box::pin(async move {
            let resp = future.await?;
            Ok(resp)
        })
    }
}
// ==

/// Builder.
#[derive(Debug)]
pub struct ReqwestBuilder<L> {
    inner: ServiceBuilder<L>,
}

impl Default for ReqwestBuilder<Identity> {
    fn default() -> Self {
        Self { inner: ServiceBuilder::new() }
    }
}

impl<L> ReqwestBuilder<L>
where
    L: Layer<Http<reqwest::Client>>,
{
    /// Add a middleware layer to the stack.
    pub fn layer<M>(self, layer: M) -> ReqwestBuilder<Stack<M, L>> {
        ReqwestBuilder { inner: self.inner.layer(layer) }
    }

    /// Build with url
    pub fn build_with_url(self, url: Url) -> L::Service {
        let transport = Http::new(url);
        self.on_transport(transport)
    }

    /// Get the service from the inner layer.
    pub fn on_transport(self, transport: Http<reqwest::Client>) -> L::Service {
        self.inner.service(transport)
    }
}

struct LoggingLayer;

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService { inner }
    }
}

struct LoggingService<S> {
    inner: S,
}

use std::{future::Future, pin::Pin};

impl<S> Service<reqwest::Request> for LoggingService<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response, Error = reqwest::Error>,
    S::Future: Send + 'static,
{
    type Response = reqwest::Response;
    type Error = reqwest::Error;
    type Future = ReqwestFuture;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: reqwest::Request) -> Self::Future {
        println!("LoggingLayer - request body is some: {:#?}", req.body().is_some());

        let future = self.inner.call(req);
        Box::pin(async move {
            let resp = future.await?;
            Ok(resp)
        })
    }
}
