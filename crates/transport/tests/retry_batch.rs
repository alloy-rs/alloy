//! Error payloads that the [`RetryBackoffLayer`] does not retry.
#![allow(missing_docs)]

use alloy_json_rpc::{
    ErrorPayload, Id, Request, RequestPacket, Response, ResponsePacket, ResponsePayload,
};
use alloy_transport::{layers::RetryBackoffLayer, TransportFut};
use serde_json::value::RawValue;
use tower::{Layer, Service};

#[tokio::test]
async fn non_retryable_batch_error_passes_the_batch_through() {
    let inner = tower::service_fn(|req: RequestPacket| {
        let reqs = req.requests().to_vec();
        Box::pin(async move {
            Ok(ResponsePacket::Batch(vec![
                Response {
                    id: reqs[0].id().clone(),
                    payload: ResponsePayload::Success(
                        RawValue::from_string("\"0x1\"".into()).unwrap(),
                    ),
                },
                Response {
                    id: reqs[1].id().clone(),
                    payload: ResponsePayload::Failure(ErrorPayload {
                        code: -32000,
                        message: "execution reverted".into(),
                        data: None,
                    }),
                },
            ]))
        }) as TransportFut<'static>
    });
    let mut service = RetryBackoffLayer::new(3, 0, 10_000).layer(inner);
    let batch: RequestPacket = (1..=2)
        .map(|id| Request::new("eth_call", Id::Number(id), ()).serialize().unwrap())
        .collect();

    let packet = service.call(batch).await.expect("the batch response is passed through");
    assert!(packet.responses()[0].is_success());
    assert_eq!(packet.responses()[1].error_code(), Some(-32000));
}
