//! Endpoint credentials must not leak into transport tracing spans.
#![cfg(all(not(target_family = "wasm"), any(feature = "reqwest", feature = "hyper")))]
#![allow(missing_docs)]

use alloy_json_rpc::RequestPacket;
use std::sync::Mutex;
use tower::Service;
use tracing_subscriber::fmt::{format::FmtSpan, MakeWriter};

static LOGS: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[test]
fn transport_spans_omit_url_secrets() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::NEW)
        .with_ansi(false)
        .with_writer(|| LOGS.make_writer())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let url: url::Url =
        "http://user:secret@127.0.0.1:8545/secret?key=secret#secret".parse().unwrap();
    #[cfg(feature = "reqwest")]
    drop(alloy_transport_http::Http::new(url.clone()).call(RequestPacket::Batch(vec![])));
    #[cfg(feature = "hyper")]
    drop(alloy_transport_http::HyperTransport::new_hyper(url).call(RequestPacket::Batch(vec![])));

    let logs = String::from_utf8(LOGS.lock().unwrap().clone()).unwrap();
    assert!(!logs.contains("secret"), "{logs}");
    for name in [
        #[cfg(feature = "reqwest")]
        "ReqwestTransport",
        #[cfg(feature = "hyper")]
        "HyperTransport",
    ] {
        let span = format!(r#"{name}{{scheme="http" host="127.0.0.1" port=8545}}"#);
        assert!(logs.contains(&span), "{logs}");
    }
}
