//! Utility functions for the node bindings.

use alloy_primitives::{hex, Address};
use k256::SecretKey;
use std::{
    borrow::Cow,
    future::Future,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    str::FromStr,
};
use tempfile;

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

/// Get the default private keys and addresses from the default mnemonic.
pub(crate) fn get_default_keys() -> (Vec<SecretKey>, Vec<Address>) {
    // From the default mnemonic "test test test test test test test test test test test
    // junk" populate the private keys and addresses.
    let private_keys = vec![
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
        "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
        "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
        "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
        "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
        "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
        "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
        "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
        "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
    ]
    .iter()
    .map(|s| {
        let key_hex = hex::decode(s).unwrap();
        SecretKey::from_bytes((&key_hex[..]).into()).unwrap()
    })
    .collect::<Vec<SecretKey>>();

    let addresses = vec![
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
        "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
        "0x90F79bf6EB2c4f870365E785982E1f101E93b906",
        "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65",
        "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc",
        "0x976EA74026E726554dB657fA54763abd0C3a0aa9",
        "0x14dC79964da2C08b23698B3D3cc7Ca32193d9955",
        "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f",
        "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720",
    ]
    .iter()
    .map(|s| Address::from_str(s).unwrap())
    .collect::<Vec<Address>>();

    (private_keys, addresses)
}

/// Runs the given closure with a temporary directory.
pub fn run_with_tempdir_sync(prefix: &str, f: impl FnOnce(PathBuf)) {
    let temp_dir = tempfile::TempDir::with_prefix(prefix).unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    f(temp_dir_path);
    #[cfg(not(windows))]
    temp_dir.close().unwrap();
}

/// Runs the given async closure with a temporary directory.
pub async fn run_with_tempdir<F, Fut>(prefix: &str, f: F)
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: Future<Output = ()>,
{
    let temp_dir = tempfile::TempDir::with_prefix(prefix).unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    f(temp_dir_path).await;
    #[cfg(not(windows))]
    temp_dir.close().unwrap();
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
