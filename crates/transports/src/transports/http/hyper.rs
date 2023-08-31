use hyper::client::{connect::Connect, Client};
use serde_json::value::RawValue;
use std::{future::Future, pin::Pin, task};
use tower::Service;

use crate::{Http, TransportError};

impl<C> Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    pub fn request(
        &self,
        req: Box<RawValue>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<RawValue>, TransportError>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            // convert the Box<RawValue> into a hyper request<B>
            let req = hyper::Request::builder()
                .method(hyper::Method::POST)
                .uri(this.url.as_str())
                .header("content-type", "application/json")
                .body(hyper::Body::from(req.get().to_owned()))
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

impl<C> Service<Box<RawValue>> for &Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.request(req)
    }
}

impl<C> Service<Box<RawValue>> for Http<Client<C>>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // hyper always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.request(req)
    }
}
