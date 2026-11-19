//! Deserialization from owned input, where map keys cannot be borrowed.

use alloy_json_rpc::{PubSubItem, Request};
use serde_json::{json, Value};

#[test]
fn request_and_pubsub_item_deserialize_from_owned_input() {
    let request = json!({"jsonrpc": "2.0", "method": "m", "params": [1], "id": 1});
    serde_json::from_value::<Request<Value>>(request.clone()).unwrap();
    serde_json::from_reader::<_, Request<Value>>(request.to_string().as_bytes()).unwrap();
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscription",
        "params": {"subscription": "0x1", "result": 1},
    });
    serde_json::from_value::<PubSubItem>(notification).unwrap();
}

#[test]
fn request_and_pubsub_item_accept_escaped_keys() {
    let request = r#"{"jsonrpc": "2.0", "method": "m", "params": [1], "\u0069d": 1}"#;
    serde_json::from_str::<Request<Value>>(request).unwrap();
    let response = r#"{"jsonrpc": "2.0", "\u0069d": 1, "result": 1}"#;
    assert!(matches!(serde_json::from_str(response).unwrap(), PubSubItem::Response(_)));
}
