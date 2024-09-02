use reqwest::{Error, Request, Response};
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// A logging layer for the HTTP transport.
#[derive(Debug, Clone)]
pub struct LoggingLayer;

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService { inner }
    }
}

/// A service that logs requests and responses.
#[derive(Debug, Clone)]
pub struct LoggingService<S> {
    inner: S,
}

impl<S> Service<Request> for LoggingService<S>
where
    S: Service<Request, Response = Response, Error = Error>,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Error;
    type Future = crate::ReqwestResponseFut<Response, Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        tracing::info!("LoggingLayer(Request) {:?}", req);

        let future = self.inner.call(req);
        Box::pin(async move {
            let resp = future.await?;
            Ok(resp)
        })
    }
}
