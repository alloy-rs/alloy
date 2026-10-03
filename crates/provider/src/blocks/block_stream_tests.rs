use super::*;
use crate::{Provider, RootProvider};
use alloy_json_rpc::RequestPacket;
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::Block;
use alloy_transport::{
    mock::{Asserter, MockTransport},
    TransportErrorKind, TransportFut,
};
use std::{collections::VecDeque, sync::Mutex};

#[derive(Clone, Copy, Debug)]
enum Reply {
    Block(u64),
    Null,
    ErrorResp,
    MissingResponse,
    BackendGone,
}

fn blocks(numbers: std::ops::RangeInclusive<u64>) -> Vec<Reply> {
    numbers.map(Reply::Block).collect()
}

fn then_block_one(failures: impl IntoIterator<Item = Reply>) -> Vec<Reply> {
    failures.into_iter().chain([Reply::Block(1)]).collect()
}

/// Answers each request with the next reply. Payloads are served through an [`Asserter`],
/// `MissingResponse` is a recoverable transport error and `BackendGone` a non-recoverable one.
fn mocked(replies: Vec<Reply>) -> (RootProvider, Arc<Mutex<VecDeque<Reply>>>) {
    let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
    let script = replies.clone();
    let asserter = Asserter::new();
    let mut transport = MockTransport::new(asserter.clone());
    let service = tower::service_fn(move |req: RequestPacket| -> TransportFut<'static> {
        let id = req.as_single().unwrap().id().clone();
        match script.lock().unwrap().pop_front() {
            Some(Reply::Block(number)) => {
                let mut block: Block = Block::default();
                block.header.inner.number = number;
                asserter.push_success(&Some(block));
            }
            Some(Reply::Null) => asserter.push_success(&None::<Block>),
            Some(Reply::ErrorResp) => asserter.push_failure_msg("unknown block"),
            Some(Reply::MissingResponse) => {
                return Box::pin(async { Err(TransportErrorKind::missing_batch_response(id)) })
            }
            Some(Reply::BackendGone) | None => {
                return Box::pin(async { Err(TransportErrorKind::backend_gone()) })
            }
        }
        tower::Service::call(&mut transport, req)
    });
    (RootProvider::new(RpcClient::new(service, true)), replies)
}

#[tokio::test]
async fn fetches_up_to_tip() {
    use Reply::*;

    let mixed_failures = [MissingResponse, Null].into_iter().cycle().take(MAX_RETRIES + 1);
    // (replies in request order, tips, yielded block numbers, unused replies)
    let cases = [
        (then_block_one([Null; MAX_RETRIES]), vec![1], vec![1], 0),
        (then_block_one([MissingResponse; MAX_RETRIES]), vec![1], vec![1], 0),
        (then_block_one([Null; MAX_RETRIES + 1]), vec![1], vec![], 1),
        (then_block_one(mixed_failures), vec![1], vec![], 1),
        (then_block_one([ErrorResp]), vec![1], vec![], 1),
        (then_block_one([BackendGone]), vec![1], vec![], 1),
        (vec![Block(1), ErrorResp, Block(2), Block(3)], vec![3, 3], vec![1, 2, 3], 0),
        (blocks(1..=3), vec![2, 1, 2, 3], vec![1, 2, 3], 0),
        (blocks(1..=15), vec![15], (1..=10).collect(), 5),
        (blocks(1..=15), vec![15, 15], (1..=15).collect(), 0),
    ];
    for (replies, tips, expected, unused) in cases {
        let case = format!("{replies:?} {tips:?}");
        let (provider, remaining) = mocked(replies);
        let new_blocks = NewBlocks::<Ethereum>::new(provider.weak_client()).with_next_yield(1);
        let yielded: Vec<_> = new_blocks
            .into_block_stream(futures::stream::iter(tips))
            .map(|block| block.header.number)
            .collect()
            .await;

        assert_eq!(yielded, expected, "{case}");
        assert_eq!(remaining.lock().unwrap().len(), unused, "{case}");
    }
}

#[tokio::test]
async fn resume_skips_blocks_produced_while_paused() {
    let (provider, remaining) = mocked(blocks(4..=8));
    let new_blocks = NewBlocks::<Ethereum>::new(provider.weak_client()).with_next_yield(1);
    let paused = new_blocks.paused.clone();
    paused.set_paused(true);
    let mut stream = Box::pin(new_blocks.into_block_stream(futures::stream::iter([5])));

    assert!(futures::poll!(stream.next()).is_pending());
    paused.set_paused(false);

    let yielded: Vec<_> = stream.map(|block| block.header.number).collect().await;
    assert_eq!(yielded, [4, 5]);
    assert_eq!(remaining.lock().unwrap().len(), 3);
}
