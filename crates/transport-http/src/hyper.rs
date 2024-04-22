use crate::Http;
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportErrorKind, TransportFut};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Buf, Bytes},
    header,
};
use hyper_util::client::legacy::{connect::Connect, Client};
use std::task;
use tower::Service;

impl<C, B> Http<Client<C, Full<B>>>
where
    C: Connect + Clone + Send + Sync + 'static,
    B: From<Bytes> + Buf + Send + 'static,
{
    /// Make a request.
    fn request_hyper(&self, req: RequestPacket) -> TransportFut<'static> {
        let this = self.clone();
        Box::pin(async move {
            let ser = req.serialize().map_err(TransportError::ser_err)?;

            // convert the Box<RawValue> into a hyper request<B>
            let body = Full::from(Bytes::from(<Box<[u8]>>::from(<Box<str>>::from(ser))));
            let req = hyper::Request::builder()
                .method(hyper::Method::POST)
                .uri(this.url.as_str())
                .header(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"))
                .body(body)
                .expect("request parts are valid");

            let resp = this.client.request(req).await.map_err(TransportErrorKind::custom)?;
            let status = resp.status();

            // Unpack data from the response body. We do this regardless of the status code, as we
            // want to return the error in the body if there is one.
            let body =
                resp.into_body().collect().await.map_err(TransportErrorKind::custom)?.to_bytes();

            if status != hyper::StatusCode::OK {
                return Err(TransportErrorKind::custom_str(&format!(
                    "HTTP error {status} with body: {}",
                    String::from_utf8_lossy(&body)
                )));
            }

            // Deser a Box<RawValue> from the body. If deser fails, return the
            // body as a string in the error. If the body is not UTF8, this will
            // fail and give the empty string in the error.
            serde_json::from_slice(&body).map_err(|err| {
                TransportError::deser_err(err, String::from_utf8_lossy(body.as_ref()))
            })
        })
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
