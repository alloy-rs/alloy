use alloy_json_rpc::{Id, Request, Response, RpcError, SubId};
use alloy_primitives::U256;
use alloy_pubsub::InFlight;
use serde_json::json;

#[derive(Debug)]
enum Expected {
    SubId(SubId),
    Forwarded,
    DeserError,
}

#[test]
fn fulfill() {
    let cases = [
        ("eth_blockNumber", r#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#, Expected::Forwarded),
        (
            "eth_subscribe",
            r#"{"jsonrpc":"2.0","id":1,"result":"0xabc"}"#,
            Expected::SubId(SubId::Number(U256::from(0xabc))),
        ),
        (
            "eth_subscribe",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"too many subscriptions"}}"#,
            Expected::Forwarded,
        ),
        ("eth_subscribe", r#"{"jsonrpc":"2.0","id":1,"result":true}"#, Expected::DeserError),
    ];

    for (method, raw, expected) in cases {
        let req = Request::new(method, Id::Number(1), json!(["newHeads"])).serialize().unwrap();
        let (in_flight, mut rx) = InFlight::new(req, 16);
        let resp: Response = serde_json::from_str(raw).unwrap();
        let serialized = serde_json::to_string(&resp).unwrap();

        let fulfilled = in_flight.fulfill(resp);

        match expected {
            Expected::SubId(sub_id) => {
                let (id, in_flight) = fulfilled.expect(raw);
                assert_eq!(id, sub_id, "{raw}");
                assert_eq!(in_flight.request.id(), &Id::Number(1), "{raw}");
                assert!(rx.try_recv().is_err(), "{raw}");
            }
            Expected::Forwarded => {
                assert!(fulfilled.is_none(), "{raw}");
                let resp = rx.try_recv().unwrap().unwrap();
                assert_eq!(serde_json::to_string(&resp).unwrap(), serialized);
            }
            Expected::DeserError => {
                assert!(fulfilled.is_none(), "{raw}");
                let err = rx.try_recv().unwrap().unwrap_err();
                assert!(matches!(err, RpcError::DeserError { .. }), "{raw}: {err:?}");
            }
        }
    }
}
