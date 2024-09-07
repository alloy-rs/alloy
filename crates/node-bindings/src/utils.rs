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

/// Extracts the value for the given key from the line of text.
/// It supports keys that end with '=' or ': '.
pub(crate) fn extract_value<'a>(key: &str, line: &'a str) -> Option<&'a str> {
    let mut key_equal = Cow::from(key);
    let mut key_colon = Cow::from(key);

    // Prepare both key variants
    if !key_equal.ends_with('=') {
        key_equal = format!("{}=", key).into();
    }
    if !key_colon.ends_with(": ") {
        key_colon = format!("{}: ", key).into();
    }

    // Try to find the key with '='
    if let Some(pos) = line.find(key_equal.as_ref()) {
        let start = pos + key_equal.len();
        let end = line[start..].find(' ').map(|i| start + i).unwrap_or(line.len());
        if start <= line.len() && end <= line.len() {
            return Some(line[start..end].trim());
        }
    }

    // If not found, try to find the key with ': '
    if let Some(pos) = line.find(key_colon.as_ref()) {
        let start = pos + key_colon.len();
        let end = line[start..].find(',').unwrap_or(line.len()); // Assuming comma or end of line
        if start <= line.len() && start + end <= line.len() {
            return Some(line[start..start + end].trim());
        }
    }

    // If neither variant matches, return None
    None
}

/// Extracts the endpoint from the given line.
pub(crate) fn extract_endpoint(key: &str, line: &str) -> Option<SocketAddr> {
    let val = extract_value(key, line)?;

    // Remove the "Some( ... )" wrapper if it exists
    let val =
        if val.starts_with("Some(") && val.ends_with(')') { &val[5..val.len() - 1] } else { val };

    val.parse::<SocketAddr>().ok()
}

#[test]
fn test_extract_http_address() {
    let line = "INFO [07-01|13:20:42.774] HTTP server started                      endpoint=127.0.0.1:8545 auth=false prefix= cors= vhosts=localhost";
    assert_eq!(extract_endpoint("endpoint=", line), Some(SocketAddr::from(([127, 0, 0, 1], 8545))));
}

#[test]
fn test_extract_udp_address() {
    let line = "Updated local ENR enr=Enr { id: Some(\"v4\"), seq: 2, NodeId: 0x04dad428038b4db230fc5298646e137564fc6861662f32bdbf220f31299bdde7, signature: \"416520d69bfd701d95f4b77778970a5c18fa86e4dd4dc0746e80779d986c68605f491c01ef39cd3739fdefc1e3558995ad2f5d325f9e1db795896799e8ee94a3\", IpV4 UDP Socket: Some(0.0.0.0:30303), IpV6 UDP Socket: None, IpV4 TCP Socket: Some(0.0.0.0:30303), IpV6 TCP Socket: None, Other Pairs: [(\"eth\", \"c984fc64ec0483118c30\"), (\"secp256k1\", \"a103aa181e8fd5df651716430f1d4b504b54d353b880256f56aa727beadd1b7a9766\")], .. }";
    assert_eq!(
        extract_endpoint("IpV4 TCP Socket: ", line),
        Some(SocketAddr::from(([0, 0, 0, 0], 30303)))
    );
}

#[test]
fn test_unused_port() {
    let port = unused_port();
    assert!(port > 0);
}
