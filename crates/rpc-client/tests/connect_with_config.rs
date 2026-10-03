//! Layers on a client built with [`ClientBuilder::connect_with_config`].
#![cfg(all(unix, feature = "ipc"))]
#![allow(missing_docs)]

use alloy_rpc_client::{ClientBuilder, ConnectionConfig};
use alloy_transport::layers::RetryBackoffLayer;
use std::os::unix::net::UnixListener;

#[tokio::test]
async fn layered_connect_with_config_keeps_pubsub_frontend() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alloy.ipc");
    let _listener = UnixListener::bind(&path).unwrap();

    let client = ClientBuilder::default()
        .layer(RetryBackoffLayer::new(3, 0, 10_000))
        .connect_with_config(path.to_str().unwrap(), ConnectionConfig::new())
        .await
        .unwrap();

    assert!(client.pubsub_frontend().is_some());
}
