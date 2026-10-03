//! Deserialization of malformed JSON-RPC messages.
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, EthNotification, Id, PubSubItem, Request, Response, ResponsePayload, RpcError,
    SubId,
};
use alloy_primitives::{bytes, Bytes, U256};
use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};

#[track_caller]
fn assert_err<T: for<'de> Deserialize<'de> + std::fmt::Debug>(json: &str, expected: &str) {
    let err = serde_json::from_str::<T>(json).unwrap_err();
    assert!(err.to_string().contains(expected), "{json}: {err}");
}

#[test]
fn request_rejects_malformed_objects() {
    let cases = [
        (r#"{"method":"m","params":[],"id":1}"#, "missing field `jsonrpc`"),
        (
            r#"{"jsonrpc":"1.0","method":"m","params":[],"id":1}"#,
            "unsupported JSON-RPC version: 1.0",
        ),
        (r#"{"jsonrpc":"2.0","params":[],"id":1}"#, "missing field `method`"),
        (r#"{"jsonrpc":"2.0","method":"m","id":1}"#, "missing field `params`"),
        (r#"{"jsonrpc":"2.0","method":"m","params":[],"id":1,"extra":1}"#, "unknown field `extra`"),
        (r#"{"jsonrpc":"2.0","method":"m","params":[],"id":1,"id":2}"#, "duplicate field `id`"),
        (
            r#"{"jsonrpc":"2.0","method":"m","params":[],"params":[],"id":1}"#,
            "duplicate field `params`",
        ),
        (
            r#"{"jsonrpc":"2.0","method":"m","method":"n","params":[],"id":1}"#,
            "duplicate field `method`",
        ),
        ("[]", "a JSON-RPC 2.0 request object"),
    ];
    for (json, expected) in cases {
        assert_err::<Request<Value>>(json, expected);
    }
}

#[test]
fn request_defaults_missing_id_and_zero_sized_params() {
    let req: Request<Value> =
        serde_json::from_str(r#"{"jsonrpc":"2.0","method":"m","params":[1]}"#).unwrap();
    assert_eq!(req.meta.id, Id::None);
    assert_eq!(req.params, json!([1]));

    let req: Request<()> =
        serde_json::from_str(r#"{"jsonrpc":"2.0","method":"m","id":7}"#).unwrap();
    assert_eq!(req.meta.method, "m");
    assert_eq!(req.meta.id, Id::Number(7));
}

#[test]
fn response_rejects_malformed_objects() {
    let cases = [
        (r#"{"jsonrpc":"2.0","id":1}"#, "missing field `result or error`"),
        (
            r#"{"jsonrpc":"2.0","id":1,"result":"0x1","error":{"code":1,"message":"m"}}"#,
            "result and error are mutually exclusive",
        ),
        (r#"{"jsonrpc":"2.0","id":1,"result":1,"result":2}"#, "duplicate field `result`"),
        (
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":1},"error":{"code":2}}"#,
            "duplicate field `error`",
        ),
        (r#"{"jsonrpc":"2.0","id":1,"id":2,"result":1}"#, "duplicate field `id`"),
        (r#"{"jsonrpc":"2.0","id":1,"error":"boom"}"#, "a JSON-RPC 2.0 error object"),
        ("[]", "a JSON-RPC response object"),
    ];
    for (json, expected) in cases {
        assert_err::<Response>(json, expected);
    }
}

#[test]
fn response_serialize_round_trips() {
    let cases = [
        r#"{"jsonrpc":"2.0","id":1,"result":{"a":[1,2]}}"#,
        r#"{"jsonrpc":"2.0","id":"x","result":null}"#,
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32000,"message":"boom","data":"0x01"}}"#,
    ];
    for json in cases {
        let response: Response = serde_json::from_str(json).unwrap();
        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            serde_json::from_str::<Value>(json).unwrap()
        );
    }
}

#[test]
fn response_deser_success_and_err() {
    let response = |json: &str| serde_json::from_str::<Response>(json).unwrap();

    let ok = response(r#"{"id":1,"result":"0x5"}"#).deser_success::<U256>().unwrap();
    assert!(matches!(ok.payload, ResponsePayload::Success(n) if n == U256::from(5)));

    let unchanged = response(r#"{"id":1,"result":"abc"}"#).deser_success::<U256>().unwrap_err();
    assert_eq!(unchanged.id, Id::Number(1));
    assert_eq!(unchanged.payload.as_success().unwrap().get(), r#""abc""#);

    let failure =
        response(r#"{"id":2,"error":{"code":3,"message":"m"}}"#).deser_success::<U256>().unwrap();
    assert_eq!(failure.payload.as_error().unwrap().code, 3);

    let err = response(r#"{"id":1,"error":{"code":3,"message":"m","data":"0x01"}}"#)
        .deser_err::<Bytes>()
        .unwrap();
    assert_eq!(err.payload.as_error().unwrap().data, Some(bytes!("01")));

    let unchanged = response(r#"{"id":1,"error":{"code":3,"message":"m","data":true}}"#)
        .deser_err::<Bytes>()
        .unwrap_err();
    assert_eq!(unchanged.id, Id::Number(1));
    assert_eq!(unchanged.payload.as_error().unwrap().data.as_ref().unwrap().get(), "true");

    let success = response(r#"{"id":1,"result":[1]}"#).deser_err::<Bytes>().unwrap();
    assert_eq!(success.payload.as_success().unwrap().get(), "[1]");
}

#[test]
fn error_payload_deserialization() {
    let payload: ErrorPayload = serde_json::from_str(r#"{"code":-1,"other":true}"#).unwrap();
    assert_eq!(payload.code, -1);
    assert_eq!(payload.message, "");
    assert!(payload.data.is_none());

    let cases = [
        (r#"{"message":"m"}"#, "missing field `code`"),
        (r#"{"code":1,"code":2}"#, "duplicate field `code`"),
        (r#"{"code":1,"message":"a","message":"b"}"#, "duplicate field `message`"),
        (r#"{"code":1,"data":1,"data":2}"#, "duplicate field `data`"),
    ];
    for (json, expected) in cases {
        assert_err::<ErrorPayload>(json, expected);
    }
}

#[test]
fn error_payload_deser_data() {
    let payload = |json: &str| serde_json::from_str::<ErrorPayload>(json).unwrap();

    let data =
        payload(r#"{"code":3,"message":"m","data":"0x0102"}"#).deser_data::<Bytes>().unwrap();
    assert_eq!(data.data, Some(bytes!("0102")));
    assert_eq!((data.code, data.message.as_ref()), (3, "m"));

    for json in [r#"{"code":3,"message":"m"}"#, r#"{"code":3,"message":"m","data":{"a":1}}"#] {
        let unchanged = payload(json).deser_data::<Bytes>().unwrap_err();
        assert_eq!(
            serde_json::to_value(&unchanged).unwrap(),
            serde_json::to_value(payload(json)).unwrap()
        );
    }
}

#[test]
fn error_payload_revert_data_is_found_in_nested_objects() {
    let cases = [
        (json!({"originalError": {"data": "0x0102"}}), "execution reverted", Some(bytes!("0102"))),
        (json!({"a": 1, "b": {"c": {"d": "0x03"}}}), "reverted", Some(bytes!("03"))),
        (json!({"a": {"b": 1}}), "execution reverted", None),
    ];
    for (data, message, expected) in cases {
        let payload = ErrorPayload {
            code: 3,
            message: message.into(),
            data: Some(RawValue::from_string(data.to_string()).unwrap()),
        };
        assert_eq!(payload.as_revert_data(), expected, "{data}");
    }
}

#[test]
fn pubsub_item_rejects_malformed_messages() {
    let cases = [
        (r#"{"jsonrpc":"2.0","id":1}"#, "missing `result` or `error` field in response"),
        (
            r#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0x1","result":1},"error":{"code":1,"message":"m"}}"#,
            "unexpected `error` field in subscription notification",
        ),
        (r#"{"id":1,"id":2,"result":1}"#, "duplicate field `id`"),
        (r#"{"id":1,"result":1,"result":2}"#, "duplicate field `result`"),
        (
            r#"{"params":{"subscription":"0x1","result":1},"params":{"subscription":"0x1","result":2}}"#,
            "duplicate field `params`",
        ),
        (r#"{"id":1,"error":{"code":1},"error":{"code":2}}"#, "duplicate field `error`"),
        ("[]", "a JSON-RPC response or an Ethereum-style notification"),
    ];
    for (json, expected) in cases {
        assert_err::<PubSubItem>(json, expected);
    }
}

#[test]
fn pubsub_item_subscription_ids() {
    let cases = [
        (json!("0x1f"), SubId::Number(U256::from(0x1f))),
        (json!(31), SubId::Number(U256::from(31))),
        (json!("abc"), SubId::String("abc".into())),
        (json!("0xzz"), SubId::String("0xzz".into())),
    ];
    for (subscription, expected) in cases {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscription",
            "params": {"subscription": subscription, "result": [1]},
        });
        let item: PubSubItem = serde_json::from_str(&json.to_string()).unwrap();
        let PubSubItem::Notification(EthNotification { subscription: id, result }) = item else {
            panic!("expected a notification for {json}");
        };
        assert_eq!(id, expected, "{json}");
        assert_eq!(result.get(), "[1]");
    }
}

#[test]
fn rpc_error_deser_err_promotes_error_payloads() {
    let source = || serde_json::from_str::<u64>("x").unwrap_err();

    let err = RpcError::<()>::deser_err(source(), r#"{"code":-32000,"message":"boom"}"#);
    let payload = err.as_error_resp().unwrap();
    assert_eq!((payload.code, payload.message.as_ref()), (-32000, "boom"));

    for text in [r#"{"message":"no code"}"#, "not json", r#""0x1""#] {
        let RpcError::DeserError { text: kept, .. } = RpcError::<()>::deser_err(source(), text)
        else {
            panic!("expected a deserialization error for {text}");
        };
        assert_eq!(kept, text);
    }
}
