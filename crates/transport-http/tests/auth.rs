//! The `Authorization` header set by the [`AuthLayer`].
#![cfg(all(feature = "jwt-auth", not(target_family = "wasm")))]
#![allow(missing_docs)]

use alloy_rpc_types_engine::JwtSecret;
use alloy_transport_http::AuthLayer;
use http_body_util::Full;
use hyper::{body::Bytes, header::AUTHORIZATION, Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    pin::pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tower::{Layer, Service};

#[test]
fn every_request_carries_a_bearer_token() {
    let secret = JwtSecret::random();
    let headers = Arc::new(Mutex::new(Vec::new()));
    let seen = headers.clone();
    let inner = tower::service_fn(move |req: Request<Full<Bytes>>| {
        seen.lock().unwrap().push(req.headers()[AUTHORIZATION].to_str().unwrap().to_owned());
        std::future::ready(Ok::<_, Infallible>(Response::new(Full::<Bytes>::default())))
    });
    let mut service = AuthLayer::new(secret).layer(inner);

    for _ in 0..2 {
        let fut = service.call(Request::new(Full::default()));
        let Poll::Ready(res) = pin!(fut).poll(&mut Context::from_waker(Waker::noop())) else {
            panic!("the inner service responds immediately");
        };
        res.unwrap();
    }

    let headers = headers.lock().unwrap();
    assert_eq!(headers.len(), 2);
    for header in headers.iter() {
        let token = header.strip_prefix("Bearer ").unwrap_or_else(|| panic!("{header}"));
        secret.validate(token).unwrap();
    }
}
