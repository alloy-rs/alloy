use crate::{Http, HttpConnect};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{
    utils::guess_local_url, BoxTransport, TransportConnect, TransportError, TransportErrorKind,
    TransportFut, TransportResult,
};
use itertools::Itertools;
use std::task;
use tower::Service;
use tracing::{debug, debug_span, instrument, trace, Instrument};
use url::Url;

/// Rexported from [`reqwest`].
pub use reqwest::Client;

/// An [`Http`] transport using [`reqwest`].
pub type ReqwestTransport = Http<Client>;

/// Connection details for a [`ReqwestTransport`].
pub type ReqwestConnect = HttpConnect<ReqwestTransport>;

impl TransportConnect for ReqwestConnect {
    fn is_local(&self) -> bool {
        guess_local_url(self.url.as_str())
    }

    async fn get_transport(&self) -> Result<BoxTransport, TransportError> {
        Ok(BoxTransport::new(Http::with_client(Client::new(), self.url.clone())))
    }
}

impl Http<Client> {
    /// Create a new [`Http`] transport.
    pub fn new(url: Url) -> Self {
        Self { client: Default::default(), url }
    }

    #[instrument(name = "request", skip_all, fields(method_names = %req.method_names().take(3).format(", ").to_string()))]
    async fn do_reqwest(self, req: RequestPacket) -> TransportResult<ResponsePacket> {
        let resp = self
            .client
            .post(self.url)
            .json(&req)
            .headers(req.headers())
            .send()
            .await
            .map_err(TransportErrorKind::custom)?;
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(crate::parse_retry_after);

        debug!(%status, "received response from server");

        // Unpack data from the response body. We do this regardless of
        // the status code, as we want to return the error in the body
        // if there is one.
        let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;

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
            .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))
    }
}

impl Service<RequestPacket> for Http<reqwest::Client> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // `reqwest` always returns `Ok(())`.
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("ReqwestTransport", url = %this.url);
        Box::pin(this.do_reqwest(req).instrument(span.or_current()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    const BODY: &str =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":429,"message":"Rate Limit Hit"}}"#;

    fn request() -> RequestPacket {
        RequestPacket::Single(Request::new("eth_chainId", Id::Number(1), ()).serialize().unwrap())
    }

    #[tokio::test]
    async fn preserves_retry_after_from_http_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_length = 0;
            let mut line = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse().unwrap();
                    }
                }
            }
            reader.read_exact(&mut vec![0_u8; content_length]).unwrap();

            let response = format!(
                "HTTP/1.1 429 Too Many Requests\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Retry-After: 52\r\n\
                 Connection: close\r\n\r\n\
                 {BODY}",
                BODY.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let mut transport = Http::new(format!("http://{address}").parse().unwrap());
        let error = transport.call(request()).await.unwrap_err();
        server.join().unwrap();

        let TransportError::Transport(error) = error else { panic!("expected transport error") };
        assert_eq!(error.retry_after(), Some(Duration::from_secs(52)));
        assert_eq!(error.as_http_error().unwrap().status, 429);
        assert_eq!(error.as_http_error().unwrap().body, BODY);
    }
}
