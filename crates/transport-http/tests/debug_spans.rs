//! Regression coverage for endpoint credentials in transport tracing spans.
#![cfg(all(not(target_family = "wasm"), any(feature = "reqwest", feature = "hyper")))]

use alloy_json_rpc::RequestPacket;
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex},
};
use tower::Service;
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, prelude::*, Layer};

#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<(String, Fields)>>>);

impl<S: Subscriber> Layer<S> for Capture {
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        self.0.lock().unwrap().push((attrs.metadata().name().to_owned(), fields));
    }
}

#[test]
fn transport_spans_exclude_url_credentials() {
    let capture = Capture::default();
    let subscriber = tracing_subscriber::registry()
        .with(capture.clone())
        .with(tracing_subscriber::filter::LevelFilter::DEBUG);
    let _guard = tracing::subscriber::set_default(subscriber);

    for endpoint in [
        "https://secret-user:secret-password@example.com:8545/secret-path?key=secret-query#secret-fragment",
        "https://user:p%40ss@example.com/%73ecret?key=%73ecret#%73ecret",
        "http://user:password@[::1]:8545/path-key?token=query-key#fragment-key",
        "https://example.com",
    ] {
        let url: url::Url = endpoint.parse().unwrap();
        capture.0.lock().unwrap().clear();
        #[cfg(feature = "reqwest")]
        {
            let mut transport = alloy_transport_http::Http::new(url.clone());
            drop(transport.call(RequestPacket::Batch(vec![])));
            assert_eq!(transport.url(), url.as_str());
        }
        #[cfg(feature = "hyper")]
        {
            let mut transport = alloy_transport_http::HyperTransport::new_hyper(url.clone());
            drop(transport.call(RequestPacket::Batch(vec![])));
            assert_eq!(transport.url(), url.as_str());
        }
        let spans = capture.0.lock().unwrap();
        for name in [
            #[cfg(feature = "reqwest")]
            "ReqwestTransport",
            #[cfg(feature = "hyper")]
            "HyperTransport",
        ] {
            let fields = &spans.iter().find(|(span, _)| span == name).unwrap().1.0;
            // Only allow non-secret endpoint fields into the transport span.
            assert_eq!(fields, &BTreeMap::from([
                ("scheme".to_owned(), url.scheme().to_owned()),
                ("host".to_owned(), format!("{:?}", url.host_str())),
            ]));
        }
    }
}
