use hyper::client::{connect::Connect, Client};
use serde_json::value::RawValue;
use std::task;
use tower::Service;

use crate::{transports::TransportRequest, Http, TransportError, TransportFut};

impl<C> Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    /// Make a request.
    fn request(&self, req: TransportRequest) -> TransportFut<'static> {
        let this = self.clone();
        Box::pin(async move {
            let ser = req.serialized()?.into_owned();
            // convert the Box<RawValue> into a hyper request<B>
            let req = hyper::Request::builder()
                .method(hyper::Method::POST)
                .uri(this.url.as_str())
                .header("content-type", "application/json")
                .body(hyper::Body::from(ser))
                .expect("request parts are valid");

            let resp = this.client.request(req).await?;

            // unpack json from the response body
            let body = hyper::body::to_bytes(resp.into_body()).await?;

            // Deser a Box<RawValue> from the body. If deser fails, return the
            // body as a string in the error. If the body is not UTF8, this will
            // fail and give the empty string in the error.
            serde_json::from_slice(body.as_ref()).map_err(|err| {
                TransportError::deser_err(err, std::str::from_utf8(body.as_ref()).unwrap_or(""))
            })
        })
    }
}

impl<C> Service<TransportRequest> for &Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: TransportRequest) -> Self::Future {
        self.request(req)
    }
}

impl<C> Service<TransportRequest> for Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: TransportRequest) -> Self::Future {
        self.request(req)
    }
}
