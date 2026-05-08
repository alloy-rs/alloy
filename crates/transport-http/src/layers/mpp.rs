//! MPP HTTP transport wrapper.
//!
//! [`MppHttp`] wraps the default reqwest [`Http`] transport and transparently
//! handles `HTTP 402 Payment Required` responses by paying the challenge via a
//! user-supplied [`PaymentProvider`] and replaying the request once with
//! `Authorization: Payment <credential>`. Any other status, or a 402 without
//! a supported challenge, is forwarded unchanged.
//!
//! Because MPP writes the `Authorization` header it is mutually exclusive
//! with [`AuthLayer`](crate::AuthLayer) and any other middleware that uses
//! the same header. Replay is bounded to a single attempt — a second 402 is
//! returned to the caller as-is.
//!
//! # Example
//!
//! ```no_run
//! use alloy_transport_http::{Http, MppHttp};
//! use mpp::{
//!     client::PaymentProvider, MppError, PaymentChallenge, PaymentCredential, PaymentPayload,
//! };
//!
//! /// A `PaymentProvider` that signs Tempo `charge` intents.
//! /// In real code, `pay()` would build, sign, and broadcast a transaction
//! /// using your wallet/signer of choice.
//! #[derive(Clone)]
//! struct MyProvider;
//!
//! impl PaymentProvider for MyProvider {
//!     fn supports(&self, method: &str, intent: &str) -> bool {
//!         method == "tempo" && intent == "charge"
//!     }
//!
//!     async fn pay(&self, challenge: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
//!         // Sign + broadcast the payment transaction here.
//!         let tx_hash = "0xdeadbeef".to_string();
//!         Ok(PaymentCredential::new(challenge.to_echo(), PaymentPayload::hash(tx_hash)))
//!     }
//! }
//!
//! # fn build() -> Result<(), Box<dyn std::error::Error>> {
//! let url = "https://paid-rpc.example.com".parse()?;
//! let transport = MppHttp::new(Http::new(url), MyProvider);
//!
//! // Plug `transport` into your `RpcClient` / `ProviderBuilder` like any
//! // other alloy HTTP transport. Payments happen transparently whenever
//! // the server returns a 402.
//! # let _ = transport;
//! # Ok(()) }
//! ```

use crate::{reqwest_transport::decode_response, Http};
use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportErrorKind, TransportFut};
use mpp::{
    client::PaymentProvider, format_authorization, parse_www_authenticate_all, PaymentChallenge,
};
use reqwest::{header::AUTHORIZATION, Client, StatusCode};
use std::task;
use tower::Service;
use tracing::{debug_span, Instrument};

pub use mpp::{
    client::PaymentProvider as Provider, PaymentChallenge as Challenge,
    PaymentCredential as Credential,
};

/// A reqwest-based HTTP transport that transparently handles MPP 402
/// challenges.
#[derive(Clone, Debug)]
pub struct MppHttp<P> {
    inner: Http<Client>,
    provider: P,
}

impl<P> MppHttp<P> {
    /// Wrap an existing [`Http<reqwest::Client>`] with MPP 402 handling.
    pub const fn new(inner: Http<Client>, provider: P) -> Self {
        Self { inner, provider }
    }

    /// Borrow the underlying transport.
    pub const fn inner(&self) -> &Http<Client> {
        &self.inner
    }
}

impl<P> MppHttp<P>
where
    P: PaymentProvider + 'static,
{
    async fn do_request(self, req: RequestPacket) -> Result<ResponsePacket, TransportError> {
        let url = self.inner.url.clone();
        let client = self.inner.client.clone();
        let headers = req.headers();

        let resp = client
            .post(url.clone())
            .json(&req)
            .headers(headers.clone())
            .send()
            .await
            .map_err(TransportErrorKind::custom)?;

        let resp = if resp.status() == StatusCode::PAYMENT_REQUIRED {
            self.maybe_pay_and_retry(resp, &client, &url, &req, headers).await?
        } else {
            resp
        };

        decode_response(resp).await
    }

    async fn maybe_pay_and_retry(
        &self,
        resp: reqwest::Response,
        client: &Client,
        url: &url::Url,
        req: &RequestPacket,
        mut headers: reqwest::header::HeaderMap,
    ) -> Result<reqwest::Response, TransportError> {
        let auth_headers: Vec<&str> = resp
            .headers()
            .get_all(reqwest::header::WWW_AUTHENTICATE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect();

        let Some(challenge) = select_challenge(&self.provider, auth_headers) else {
            return Ok(resp);
        };

        // Drain the 402 body so the connection can be reused.
        let _ = resp.bytes().await;

        let credential =
            self.provider.pay(&challenge).await.map_err(TransportErrorKind::non_retryable)?;
        let auth = format_authorization(&credential).map_err(TransportErrorKind::non_retryable)?;
        let mut value =
            reqwest::header::HeaderValue::from_str(&auth).map_err(TransportErrorKind::custom)?;
        value.set_sensitive(true);
        headers.insert(AUTHORIZATION, value);

        // Replay exactly once; we do not recurse on a second 402.
        client
            .post(url.clone())
            .json(req)
            .headers(headers)
            .send()
            .await
            .map_err(TransportErrorKind::custom)
    }
}

/// First [`PaymentChallenge`] that `provider` supports, ignoring non-Payment
/// challenges and parse failures.
fn select_challenge<'a, P: PaymentProvider>(
    provider: &P,
    headers: impl IntoIterator<Item = &'a str>,
) -> Option<PaymentChallenge> {
    parse_www_authenticate_all(headers)
        .into_iter()
        .filter_map(Result::ok)
        .find(|c| provider.supports(c.method.as_str(), c.intent.as_str()))
}

impl<P> Service<RequestPacket> for MppHttp<P>
where
    P: PaymentProvider + Clone + 'static,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("MppHttp", url = %this.inner.url);
        Box::pin(this.do_request(req).instrument(span.or_current()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{Id, Request};
    use mpp::{
        format_authorization, parse_authorization, Base64UrlJson, MppError, PaymentChallenge,
        PaymentCredential, PaymentPayload,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    // ─── Fixtures ───────────────────────────────────────────────────────────

    #[derive(Clone)]
    struct StubProvider {
        method: &'static str,
        intent: &'static str,
        pay_count: Arc<AtomicUsize>,
    }

    impl StubProvider {
        fn new(method: &'static str, intent: &'static str) -> Self {
            Self { method, intent, pay_count: Arc::new(AtomicUsize::new(0)) }
        }
        fn pay_count(&self) -> usize {
            self.pay_count.load(Ordering::SeqCst)
        }
    }

    impl PaymentProvider for StubProvider {
        fn supports(&self, method: &str, intent: &str) -> bool {
            method == self.method && intent == self.intent
        }
        async fn pay(&self, challenge: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            self.pay_count.fetch_add(1, Ordering::SeqCst);
            Ok(PaymentCredential::new(
                challenge.to_echo(),
                PaymentPayload::hash("0xdeadbeef".to_string()),
            ))
        }
    }

    #[derive(Clone)]
    struct FailingProvider;

    impl PaymentProvider for FailingProvider {
        fn supports(&self, _: &str, _: &str) -> bool {
            true
        }
        async fn pay(&self, _: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
            Err(MppError::Http("provider failure".into()))
        }
    }

    fn challenge_header(method: &str, intent: &str, id: &str) -> String {
        let request = Base64UrlJson::from_value(&serde_json::json!({"amount": "1"})).unwrap();
        PaymentChallenge::new(
            id.to_string(),
            "test".to_string(),
            method.to_string(),
            intent.to_string(),
            request,
        )
        .to_header()
        .unwrap()
    }

    /// A canned HTTP response.
    struct CannedResponse {
        status: u16,
        reason: &'static str,
        headers: Vec<(String, String)>,
        body: String,
    }

    fn ok_json() -> CannedResponse {
        CannedResponse {
            status: 200,
            reason: "OK",
            headers: vec![("content-type".into(), "application/json".into())],
            body: r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#.to_string(),
        }
    }

    fn payment_required(method: &str, intent: &str, id: &str) -> CannedResponse {
        CannedResponse {
            status: 402,
            reason: "Payment Required",
            headers: vec![("www-authenticate".into(), challenge_header(method, intent, id))],
            body: "pay required".to_string(),
        }
    }

    /// Bind a TCP listener and serve `responses` in order, recording each
    /// request's `Authorization` header. The server stops after `responses`
    /// is exhausted.
    async fn run_server(
        responses: Vec<CannedResponse>,
    ) -> (url::Url, Arc<tokio::sync::Mutex<Vec<Option<String>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url: url::Url = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
        let auth_log: Arc<tokio::sync::Mutex<Vec<Option<String>>>> = Default::default();

        let log = auth_log.clone();
        tokio::spawn(async move {
            for resp in responses {
                let (mut sock, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => return,
                };

                // Read request: headers + content-length body.
                let mut buf = vec![0u8; 8192];
                let mut total = 0;
                let mut header_end = None;
                while header_end.is_none() {
                    let n = match sock.read(&mut buf[total..]).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    total += n;
                    if let Some(pos) = buf[..total].windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = Some(pos + 4);
                    }
                }
                let header_end = header_end.unwrap();
                let head = std::str::from_utf8(&buf[..header_end]).unwrap_or("").to_string();

                // Capture Authorization header (if any), preserving casing of
                // the value so scheme matching works.
                let auth = head.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("authorization") {
                        Some(value.trim().to_string())
                    } else {
                        None
                    }
                });
                log.lock().await.push(auth);

                // Drain body (content-length).
                let content_length = head.lines().find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length:")
                        .and_then(|rest| rest.trim().parse::<usize>().ok())
                });
                if let Some(len) = content_length {
                    let already = total - header_end;
                    if already < len {
                        let mut remaining = vec![0u8; len - already];
                        let _ = sock.read_exact(&mut remaining).await;
                    }
                }

                // Write canned response.
                let mut out = format!("HTTP/1.1 {} {}\r\n", resp.status, resp.reason);
                for (k, v) in &resp.headers {
                    out.push_str(&format!("{k}: {v}\r\n"));
                }
                out.push_str(&format!(
                    "content-length: {}\r\nconnection: close\r\n\r\n",
                    resp.body.len()
                ));
                let _ = sock.write_all(out.as_bytes()).await;
                let _ = sock.write_all(resp.body.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });

        (url, auth_log)
    }

    fn rpc_request() -> RequestPacket {
        RequestPacket::Single(
            Request::new("eth_blockNumber", Id::Number(1), ()).serialize().unwrap(),
        )
    }

    async fn call<P: PaymentProvider + Clone + 'static>(
        url: url::Url,
        provider: P,
    ) -> Result<ResponsePacket, TransportError> {
        let mut t = MppHttp::new(Http::new(url), provider);
        t.call(rpc_request()).await
    }

    // ─── select_challenge ───────────────────────────────────────────────────

    #[test]
    fn select_picks_first_supported() {
        let p = StubProvider::new("tempo", "charge");
        let h1 = challenge_header("stripe", "charge", "id-1");
        let h2 = challenge_header("tempo", "charge", "id-2");
        let chosen = select_challenge(&p, [h1.as_str(), h2.as_str()]).unwrap();
        assert_eq!(chosen.id, "id-2");
    }

    #[test]
    fn select_returns_none_when_unsupported() {
        let p = StubProvider::new("tempo", "charge");
        let h = challenge_header("stripe", "charge", "id-x");
        assert!(select_challenge(&p, [h.as_str()]).is_none());
    }

    #[test]
    fn select_ignores_non_payment_schemes() {
        let p = StubProvider::new("tempo", "charge");
        let h = format!("Bearer realm=x, {}", challenge_header("tempo", "charge", "id-z"));
        assert_eq!(select_challenge(&p, [h.as_str()]).unwrap().id, "id-z");
    }

    // ─── End-to-end via in-process server ──────────────────────────────────

    #[tokio::test]
    async fn passes_through_non_402() {
        let (url, log) = run_server(vec![ok_json()]).await;
        let provider = StubProvider::new("tempo", "charge");

        let resp = call(url, provider.clone()).await.expect("ok response");
        assert!(matches!(resp, ResponsePacket::Single(_)));
        assert_eq!(provider.pay_count(), 0);
        assert_eq!(log.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn pays_and_retries_on_402() {
        let (url, log) =
            run_server(vec![payment_required("tempo", "charge", "ch-1"), ok_json()]).await;
        let provider = StubProvider::new("tempo", "charge");

        let resp = call(url, provider.clone()).await.expect("ok after retry");
        assert!(matches!(resp, ResponsePacket::Single(_)));
        assert_eq!(provider.pay_count(), 1);

        let log = log.lock().await;
        assert_eq!(log.len(), 2);
        assert!(log[0].is_none(), "first request has no Authorization");
        let auth = log[1].as_ref().expect("retry must carry Authorization");
        assert!(auth.starts_with("Payment "), "scheme must be Payment, got: {auth}");
        let cred = parse_authorization(auth).expect("authorization parses");
        assert_eq!(cred.challenge.id, "ch-1");
        let _ = format_authorization(&cred).expect("re-format");
    }

    #[tokio::test]
    async fn passes_through_402_with_unsupported_challenge() {
        let (url, log) = run_server(vec![payment_required("stripe", "charge", "ch-x")]).await;
        let provider = StubProvider::new("tempo", "charge");

        let err = call(url, provider.clone()).await.expect_err("402 propagates as error");
        match err {
            alloy_json_rpc::RpcError::Transport(k) if k.is_http_error() => {
                assert_eq!(k.as_http_error().unwrap().status, 402);
            }
            other => panic!("expected http error, got {other:?}"),
        }
        assert_eq!(provider.pay_count(), 0);
        assert_eq!(log.lock().await.len(), 1, "no replay");
    }

    #[tokio::test]
    async fn second_402_after_replay_is_returned_as_is() {
        let (url, log) = run_server(vec![
            payment_required("tempo", "charge", "ch-z"),
            payment_required("tempo", "charge", "ch-z"),
        ])
        .await;
        let provider = StubProvider::new("tempo", "charge");

        let err = call(url, provider.clone()).await.expect_err("second 402 propagates");
        let kind = match err {
            alloy_json_rpc::RpcError::Transport(k) => k,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(kind.is_http_error() && kind.as_http_error().unwrap().status == 402);
        assert_eq!(provider.pay_count(), 1, "exactly one payment attempt");
        assert_eq!(log.lock().await.len(), 2, "first send + one replay only");
    }

    #[tokio::test]
    async fn provider_failure_is_non_retryable() {
        let (url, _) = run_server(vec![payment_required("tempo", "charge", "ch-f")]).await;

        let err = call(url, FailingProvider).await.expect_err("provider error bubbles up");
        let kind = match err {
            alloy_json_rpc::RpcError::Transport(k) => k,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(kind.is_non_retryable(), "got: {kind:?}");
    }
}
