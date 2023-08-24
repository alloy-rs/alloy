use serde_json::value::RawValue;
use std::{future::Future, pin::Pin, task};
use tower::Service;

use crate::{Http, TransportError};

impl Service<Box<RawValue>> for Http<reqwest::Client> {
    type Response = Box<RawValue>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.client.poll_ready(cx).map_err(Into::into)
    }

    #[inline]
    fn call(&mut self, req: Box<RawValue>) -> Self::Future {
        let replacement = self.client.clone();
        let client = std::mem::replace(&mut self.client, replacement);

        let url = self.url.clone();

        Box::pin(async move {
            let resp = client.post(url).json(&req).send().await?;
            let json = resp.text().await?;

            RawValue::from_string(json).map_err(|err| TransportError::deser_err(err, ""))
        })
    }
}
