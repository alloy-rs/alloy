//! The `Authorization` header set by the [`AuthLayer`] on a [`RequestPacket`] transport.
#![cfg(all(feature = "jwt-auth-packet", not(target_family = "wasm")))]
#![allow(missing_docs)]

use alloy_json_rpc::{Id, Request, RequestPacket, SerializedRequest};
use alloy_rpc_types_engine::JwtSecret;
use alloy_transport::{BoxTransport, TransportFut};
use alloy_transport_http::AuthLayer;
use http::{header::AUTHORIZATION, HeaderValue};
use std::sync::{Arc, Mutex};
use tower::{Layer, Service};

fn request(id: u64) -> SerializedRequest {
    Request::new("eth_chainId", Id::Number(id), ()).serialize().unwrap()
}

/// Sends `packet` through an [`AuthLayer`] and returns the `Authorization` values of the
/// packet's combined headers, which is what the HTTP transports send.
fn sent_authorization(secret: JwtSecret, packet: RequestPacket) -> Vec<String> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let inner = {
        let seen = seen.clone();
        tower::service_fn(move |req: RequestPacket| {
            *seen.lock().unwrap() = req
                .headers()
                .get_all(AUTHORIZATION)
                .iter()
                .map(|v| v.to_str().unwrap().to_owned())
                .collect();
            // The headers are observed when the request is handed over, the response is not
            // needed.
            Box::pin(std::future::pending()) as TransportFut<'static>
        })
    };

    // Client transports are boxed, which requires the layered service to be `Clone`.
    let mut transport = BoxTransport::new(AuthLayer::new(secret).layer(inner));
    drop(transport.call(packet));

    let seen = seen.lock().unwrap().clone();
    seen
}

fn assert_one_valid_token(secret: JwtSecret, values: &[String]) {
    assert_eq!(values.len(), 1, "{values:?}");
    let token = values[0].strip_prefix("Bearer ").unwrap_or_else(|| panic!("{}", values[0]));
    secret.validate(token).unwrap();
}

#[test]
fn single_request_carries_a_bearer_token() {
    let secret = JwtSecret::random();
    let values = sent_authorization(secret, request(1).into());
    assert_one_valid_token(secret, &values);
}

#[test]
fn batch_carries_exactly_one_bearer_token() {
    let secret = JwtSecret::random();
    let mut stale = request(1);
    stale.headers_mut().insert(AUTHORIZATION, HeaderValue::from_static("Bearer stale"));
    let mut other = request(2);
    other.headers_mut().insert("x-custom", HeaderValue::from_static("kept"));
    let packet = RequestPacket::Batch(vec![stale, other, request(3)]);

    let values = sent_authorization(secret, packet);
    assert_one_valid_token(secret, &values);
}

#[cfg(feature = "reqwest")]
#[tokio::test]
async fn reqwest_transport_sends_the_token() {
    use alloy_transport_http::Http;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        let mut buf = [0; 1024];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).await.unwrap();
            assert_ne!(n, 0, "connection closed before the headers ended");
            head.extend_from_slice(&buf[..n]);
        }
        let body = r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(head).unwrap()
    });

    let secret = JwtSecret::random();
    let mut transport = AuthLayer::new(secret).layer(Http::new(url));
    transport.call(request(1).into()).await.unwrap();

    let head = server.await.unwrap();
    let values: Vec<String> = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.trim().to_owned())
        .collect();
    assert_one_valid_token(secret, &values);
}
