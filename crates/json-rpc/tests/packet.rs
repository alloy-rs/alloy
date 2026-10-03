//! Request and response packet behaviour.
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, Id, Request, RequestPacket, Response, ResponsePacket, ResponsePayload,
    SerializedRequest,
};
use alloy_primitives::map::HashSet;
use http::{HeaderName, HeaderValue};
use serde_json::{value::RawValue, Value};

#[derive(Debug, PartialEq, Eq)]
enum Shape {
    Single(Id),
    Batch(Vec<Id>),
}

fn shape(packet: &ResponsePacket) -> Shape {
    match packet {
        ResponsePacket::Single(response) => Shape::Single(response.id.clone()),
        ResponsePacket::Batch(batch) => {
            Shape::Batch(batch.iter().map(|response| response.id.clone()).collect())
        }
    }
}

fn ok(id: u64) -> Response {
    Response {
        id: Id::Number(id),
        payload: ResponsePayload::Success(RawValue::from_string(id.to_string()).unwrap()),
    }
}

fn err(id: u64, code: i64) -> Response {
    Response {
        id: Id::Number(id),
        payload: ResponsePayload::Failure(ErrorPayload {
            code,
            message: "boom".into(),
            data: None,
        }),
    }
}

fn request(id: u64, method: &'static str) -> SerializedRequest {
    Request::new(method, Id::Number(id), ()).serialize().unwrap()
}

#[test]
fn response_packet_deserializes_object_or_array() {
    let cases = [
        (r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#, Shape::Single(Id::Number(1))),
        (
            r#"[{"jsonrpc":"2.0","id":2,"result":"0x2"},{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"boom"}}]"#,
            Shape::Batch(vec![Id::Number(2), Id::Number(1)]),
        ),
        (
            r#"[{"jsonrpc":"2.0","id":"a","result":null}]"#,
            Shape::Batch(vec![Id::String("a".into())]),
        ),
        ("[]", Shape::Batch(vec![])),
    ];
    for (json, expected) in cases {
        let packet: ResponsePacket = serde_json::from_str(json).unwrap();
        assert_eq!(shape(&packet), expected, "{json}");
    }

    let invalid = [
        ("null", "a single response or a batch of responses"),
        (r#""0x1""#, "a single response or a batch of responses"),
        ("[1]", "a JSON-RPC response object"),
        (r#"[{"jsonrpc":"2.0","id":1}]"#, "missing field `result or error`"),
        (r#"{"jsonrpc":"2.0","id":1}"#, "missing field `result or error`"),
    ];
    for (json, expected) in invalid {
        let err = serde_json::from_str::<ResponsePacket>(json).unwrap_err();
        assert!(err.to_string().contains(expected), "{json}: {err}");
    }
}

#[test]
fn response_packet_error_accessors() {
    let cases = [
        (ResponsePacket::Single(ok(1)), true, false, vec![]),
        (ResponsePacket::Single(err(1, -1)), false, true, vec![-1]),
        (ResponsePacket::Batch(vec![]), true, false, vec![]),
        (ResponsePacket::Batch(vec![ok(1), ok(2)]), true, false, vec![]),
        (
            ResponsePacket::Batch(vec![ok(1), err(2, -2), ok(3), err(4, -4)]),
            false,
            true,
            vec![-2, -4],
        ),
        (ResponsePacket::Batch(vec![err(1, -1), err(2, -2)]), false, true, vec![-1, -2]),
    ];
    for (packet, is_success, is_error, codes) in cases {
        let desc = format!("{:?}", shape(&packet));
        assert_eq!(packet.is_success(), is_success, "{desc}");
        assert_eq!(packet.is_error(), is_error, "{desc}");
        assert_eq!(packet.as_error().map(|e| e.code), codes.first().copied(), "{desc}");
        assert_eq!(packet.first_error_code(), codes.first().copied(), "{desc}");
        assert_eq!(packet.iter_errors().map(|e| e.code).collect::<Vec<_>>(), codes, "{desc}");
    }
}

#[test]
fn response_packet_from_responses_collapses_one_to_single() {
    let cases = [
        (vec![], Shape::Batch(vec![])),
        (vec![ok(1)], Shape::Single(Id::Number(1))),
        (vec![ok(1), err(2, -2), ok(3)], Shape::Batch(vec![1.into(), 2.into(), 3.into()])),
    ];
    for (responses, expected) in cases {
        let collected: ResponsePacket = responses.clone().into_iter().collect();
        assert_eq!(shape(&collected), expected);
        assert_eq!(shape(&ResponsePacket::from(responses)), expected);
    }
}

#[test]
fn response_packet_responses_by_ids() {
    let ids = |ids: &[u64]| ids.iter().copied().map(Id::Number).collect::<HashSet<_>>();
    let cases = [
        (ResponsePacket::Single(ok(1)), ids(&[1, 2]), vec![1]),
        (ResponsePacket::Single(ok(3)), ids(&[1, 2]), vec![]),
        (ResponsePacket::Batch(vec![ok(1), err(2, -2), ok(3)]), ids(&[3, 2, 9]), vec![2, 3]),
        (ResponsePacket::Batch(vec![ok(1), ok(2), ok(1)]), ids(&[1]), vec![1, 1]),
        (ResponsePacket::Batch(vec![ok(1)]), ids(&[]), vec![]),
    ];
    for (packet, ids, expected) in cases {
        let mut found: Vec<_> =
            packet.responses_by_ids(&ids).iter().map(|r| r.id.clone()).collect();
        found.sort();
        let expected: Vec<_> = expected.into_iter().map(Id::Number).collect();
        assert_eq!(found, expected, "{:?}", shape(&packet));
    }
}

#[test]
fn request_packet_push_promotes_single_to_batch() {
    let wire = |packet: RequestPacket| -> Value {
        serde_json::from_str(packet.serialize().unwrap().get()).unwrap()
    };

    let single = RequestPacket::from(request(0, "eth_chainId"));
    assert_eq!(single.len(), 1);
    assert_eq!(wire(single)["id"], 0);

    let mut packet = RequestPacket::from(request(0, "eth_chainId"));
    packet.push(request(1, "eth_blockNumber"));
    let ids: Vec<_> = packet.as_batch().unwrap().iter().map(|r| r.id().clone()).collect();
    assert_eq!(ids, [Id::Number(0), Id::Number(1)]);
    let wire_ids: Vec<_> =
        wire(packet).as_array().unwrap().iter().map(|r| r["id"].clone()).collect();
    assert_eq!(wire_ids, [0, 1]);

    let mut packet = RequestPacket::with_capacity(1);
    assert!(packet.is_empty());
    packet.push(request(0, "eth_chainId"));
    assert_eq!(packet.len(), 1);
    assert_eq!(wire(packet).as_array().map(Vec::len), Some(1));
}

#[test]
fn request_packet_headers_merge_in_request_order() {
    let with_headers = |id, headers: &[(&'static str, &'static str)]| {
        let mut req = request(id, "eth_call");
        for &(name, value) in headers {
            req.headers_mut()
                .append(HeaderName::from_static(name), HeaderValue::from_static(value));
        }
        req
    };

    let packet: RequestPacket = [
        with_headers(0, &[("x-a", "1"), ("x-a", "2")]),
        with_headers(1, &[]),
        with_headers(2, &[("x-b", "3"), ("x-a", "4")]),
    ]
    .into_iter()
    .collect();

    let headers = packet.headers();
    let values =
        |name| headers.get_all(name).iter().map(|v| v.to_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(values("x-a"), ["1", "2", "4"]);
    assert_eq!(values("x-b"), ["3"]);
    assert_eq!(headers.len(), 4);

    assert!(RequestPacket::from(request(0, "eth_call")).headers().is_empty());
}

#[test]
fn request_packet_subscription_request_ids() {
    let custom_sub = || {
        let mut req = request(2, "eth_subscribeCustom");
        req.set_is_subscription();
        req
    };

    let cases = [
        (RequestPacket::from(request(0, "eth_subscribe")), vec![0]),
        (RequestPacket::from(request(0, "eth_blockNumber")), vec![]),
        (RequestPacket::from(custom_sub()), vec![2]),
        (
            [request(0, "eth_subscribe"), request(1, "eth_unsubscribe"), custom_sub()]
                .into_iter()
                .collect(),
            vec![0, 2],
        ),
    ];
    for (packet, expected) in cases {
        let mut ids: Vec<_> = packet.subscription_request_ids().into_iter().cloned().collect();
        ids.sort();
        assert_eq!(ids, expected.into_iter().map(Id::Number).collect::<Vec<_>>());
    }
}
