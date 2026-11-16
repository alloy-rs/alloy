use alloy_json_rpc::{Id, Request, Response, RpcError, SubId};
use alloy_primitives::U256;
use alloy_pubsub::InFlight;
use serde_json::json;
use test_case::test_case;

#[derive(Debug)]
enum Expected {
    SubId(SubId),
    Forwarded,
    DeserError,
}

#[test_case(
    "eth_blockNumber",
    r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#,
    Expected::Forwarded;
    "non subscribe result is forwarded"
)]
#[test_case(
    "eth_subscribe",
    r#"{"jsonrpc":"2.0","id":1,"result":"0xabc"}"#,
    Expected::SubId(SubId::Number(U256::from(0xabc)));
    "subscribe result yields sub id"
)]
#[test_case(
    "eth_subscribe",
    r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"too many subscriptions"}}"#,
    Expected::Forwarded;
    "subscribe error is forwarded"
)]
#[test_case(
    "eth_subscribe",
    r#"{"jsonrpc":"2.0","id":1,"result":true}"#,
    Expected::DeserError;
    "subscribe invalid result is deser error"
)]
fn fulfill(method: &'static str, raw: &str, expected: Expected) {
    let req = Request::new(method, Id::Number(1), json!(["newHeads"])).serialize().unwrap();
    let (in_flight, mut rx) = InFlight::new(req, 16);
    let resp: Response = serde_json::from_str(raw).unwrap();
    let serialized = serde_json::to_string(&resp).unwrap();

    let fulfilled = in_flight.fulfill(resp);

    match expected {
        Expected::SubId(sub_id) => {
            let (id, in_flight) = fulfilled.expect(raw);
            assert_eq!(id, sub_id);
            assert_eq!(in_flight.request.id(), &Id::Number(1));
            assert!(rx.try_recv().is_err());
        }
        Expected::Forwarded => {
            assert!(fulfilled.is_none());
            let resp = rx.try_recv().unwrap().unwrap();
            assert_eq!(serde_json::to_string(&resp).unwrap(), serialized);
        }
        Expected::DeserError => {
            assert!(fulfilled.is_none());
            let err = rx.try_recv().unwrap().unwrap_err();
            assert!(matches!(err, RpcError::DeserError { .. }), "{raw}: {err:?}");
        }
    }
}
