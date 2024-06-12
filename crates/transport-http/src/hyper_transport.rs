use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, TransportConnect, TransportError, TransportErrorKind, TransportFut,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Buf, Bytes},
    header,
};
use hyper_util::client::legacy::{connect::Connect, Client};
use std::task;
use tower::Service;
use tracing::{debug, debug_span, trace, Instrument};

/// A [`hyper`] HTTP client.
pub type HyperClient = hyper_util::client::legacy::Client<
    hyper_util::client::legacy::connect::HttpConnector,
    http_body_util::Full<::hyper::body::Bytes>,
>;

/// An [`Http`] transport using [`hyper`].
pub type HyperTransport = Http<HyperClient>;

/// Connection details for a [`HyperTransport`].
pub type HyperConnect = HttpConnect<HyperTransport>;

impl TransportConnect for HyperConnect {
    type Transport = HyperTransport;

    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    fn get_transport<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, Self::Transport, TransportError> {
        let executor = hyper_util::rt::TokioExecutor::new();

        let client = hyper_util::client::legacy::Client::builder(executor).build_http();

        Box::pin(async move { Ok(Http::with_client(client, self.url.clone())) })
    }
}

impl<C, B> Http<Client<C, Full<B>>>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static,
{
    /// Make a request.
    fn request_hyper(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        let span = debug_span!("HyperTransport", url = %self.url);
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
                    .expect("request parts are valid");

                let resp = this.client.request(req).await.map_err(TransportErrorKind::custom)?;
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

impl<C, B> Service<RequestPacket> for &Http<Client<C, Full<B>>>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_hyper(req)
    }
}

impl<C, B> Service<RequestPacket> for Http<Client<C, Full<B>>>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_hyper(req)
    }
}
