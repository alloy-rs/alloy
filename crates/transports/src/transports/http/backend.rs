use serde_json::value::RawValue;
use std::task;
use tower::Service;

use crate::{HttpTransport, TransportError, TransportFut};

impl HttpTransport<reqwest::Client> {
    /// Make a request.
    fn request(&self, req: Box<RawValue>) -> TransportFut<'static> {
        let this = self.clone();
        Box::pin(async move {
            let resp = this.client.post(this.url).json(&req).send().await?;
            let json = resp.text().await?;

            RawValue::from_string(json).map_err(|err| TransportError::deser_err(err, ""))
        })
    }
}

impl Service<Box<RawValue>> for HttpTransport<reqwest::Client> {
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // reqwest always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.request(req)
    }
}

impl Service<Box<RawValue>> for &HttpTransport<reqwest::Client> {
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // reqwest always returns ok
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        self.request(req)
    }
}
