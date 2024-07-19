//! Utility functions for the node bindings.

use std::{
    borrow::Cow,
    net::{SocketAddr, TcpListener},
};

/// A bit of hack to find an unused TCP port.
///
/// Does not guarantee that the given port is unused after the function exists, just that it was
/// unused before the function started (i.e., it does not reserve a port).
pub(crate) fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to create TCP listener to find unused port");

    let local_addr =
        listener.local_addr().expect("Failed to read TCP listener local_addr to find unused port");
    local_addr.port()
}

/// Extracts the value for the given key and line.
pub(crate) fn extract_value<'a>(key: &str, line: &'a str) -> Option<&'a str> {
    let mut key = Cow::from(key);
    if !key.ends_with('=') {
        key = format!("{}=", key).into();
    }
    line.find(key.as_ref()).map(|pos| {
        let start = pos + key.len();
        let end = line[start..].find(' ').map(|i| start + i).unwrap_or(line.len());
        line[start..end].trim()
    })
}

/// Extracts the endpoint from the given line.
pub(crate) fn extract_endpoint(line: &str) -> Option<SocketAddr> {
    let val = extract_value("endpoint=", line)?;
    val.parse::<SocketAddr>().ok()
}

#[test]
fn test_extract_address() {
    let line = "INFO [07-01|13:20:42.774] HTTP server started                      endpoint=127.0.0.1:8545 auth=false prefix= cors= vhosts=localhost";
    assert_eq!(extract_endpoint(line), Some(SocketAddr::from(([127, 0, 0, 1], 8545))));
}
