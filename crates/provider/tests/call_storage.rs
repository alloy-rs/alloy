//! Retained storage and ownership of ordinary `EthCall` futures.

use alloy_eips::BlockId;
use alloy_network::{Ethereum, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Caller, EthCall, EthCallManyParams, EthCallParams, ProviderCall};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_eth::{state::AccountOverride, BlockOverrides, TransactionRequest};
use alloy_transport::{mock::Asserter, TransportErrorKind, TransportResult};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    future::{poll_fn, Future, IntoFuture},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::sync::oneshot;

thread_local! {
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}

struct MeasuredAllocator;

#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

fn account(bytes: isize) {
    let _ = LIVE_BYTES.try_with(|total| total.set(total.get() + bytes));
}

// SAFETY: all operations forward the original pointer and layout to System.
// Thread-local accounting neither allocates nor dereferences an allocation.
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            account(layout.size() as isize);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            account(layout.size() as isize);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        account(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            account(size as isize - layout.size() as isize);
        }
        result
    }
}

#[derive(Debug)]
struct Gate {
    method: &'static str,
    expected_params: String,
    ready: AtomicBool,
    calls: AtomicUsize,
    active: AtomicUsize,
    dropped: AtomicUsize,
}

impl Gate {
    fn new(method: &'static str) -> Arc<Self> {
        let params = EthCallParams::<Ethereum>::new(request())
            .with_block(block())
            .with_overrides(
                [(Address::with_last_byte(3), account_override())].into_iter().collect(),
            )
            .with_block_overrides(block_overrides());
        Arc::new(Self {
            method,
            expected_params: serde_json::to_string(&params).unwrap(),
            ready: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
        })
    }
}

struct ActiveReply(Arc<Gate>);

impl Drop for ActiveReply {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug)]
struct GatedCaller(Arc<Gate>);

impl GatedCaller {
    fn start(
        &self,
        params: EthCallParams<Ethereum>,
        method: &'static str,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        assert_eq!(method, self.0.method);
        assert_eq!(serde_json::to_string(&params).unwrap(), self.0.expected_params);
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        self.0.active.fetch_add(1, Ordering::SeqCst);
        let active = ActiveReply(self.0.clone());
        Ok(ProviderCall::BoxedFuture(Box::pin(poll_fn(move |_| {
            if active.0.ready.load(Ordering::SeqCst) {
                Poll::Ready(Ok(Bytes::from_static(&[0x12, 0x34])))
            } else {
                Poll::Pending
            }
        }))))
    }
}

impl Caller<Ethereum, Bytes> for GatedCaller {
    fn call(
        &self,
        params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        self.start(params, "eth_call")
    }

    fn estimate_gas(
        &self,
        params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        self.start(params, "eth_estimateGas")
    }

    fn call_many(
        &self,
        _params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Bytes>> {
        panic!("ordinary calls must not become callMany")
    }
}

fn request() -> TransactionRequest {
    TransactionRequest::default()
        .with_from(Address::with_last_byte(1))
        .with_to(Address::with_last_byte(2))
        .with_input(Bytes::from_static(&[0xaa, 0xbb, 0xcc, 0xdd]))
        .with_value(U256::from(123))
        .with_gas_limit(456_789)
        .with_gas_price(987_654)
}

const fn block() -> BlockId {
    BlockId::hash_canonical(B256::repeat_byte(0x42))
}

fn account_override() -> AccountOverride {
    AccountOverride { balance: Some(U256::from(321)), ..Default::default() }
}

fn block_overrides() -> BlockOverrides {
    BlockOverrides { time: Some(123_456), ..Default::default() }
}

fn call(gate: &Arc<Gate>) -> EthCall<Ethereum, Bytes> {
    EthCall::new(GatedCaller(gate.clone()), gate.method, request())
        .block(block())
        .account_override(Address::with_last_byte(3), account_override())
        .with_block_overrides(block_overrides())
}

fn pending_storage(method: &'static str, cancel: bool) {
    const CALLS: usize = 64;
    let gate = Gate::new(method);
    // A borrowed, non-Send mapper must remain supported by the outer future.
    let mapped = Rc::new(Cell::new(0));
    let mut futures = Vec::with_capacity(CALLS);
    let mut context = Context::from_waker(futures::task::noop_waker_ref());
    let before = LIVE_BYTES.with(Cell::get);
    for _ in 0..CALLS {
        let mut future = Box::pin(
            call(&gate)
                .map_resp(|bytes| {
                    mapped.set(mapped.get() + 1);
                    (bytes, 17)
                })
                .into_future(),
        );
        assert!(future.as_mut().poll(&mut context).is_pending());
        futures.push(future);
    }
    // No tasks, yields or cross-thread transfers occur in this interval.
    // Count nested allocations, including any boxed initialization storage.
    let retained = LIVE_BYTES.with(Cell::get) - before;
    assert_eq!(gate.calls.load(Ordering::SeqCst), CALLS);
    assert_eq!(gate.active.load(Ordering::SeqCst), CALLS);
    assert_eq!(mapped.get(), 0);
    if !cancel {
        gate.ready.store(true, Ordering::SeqCst);
        for future in &mut futures {
            let Poll::Ready(Ok(actual)) = future.as_mut().poll(&mut context) else {
                panic!("released call must complete")
            };
            assert_eq!(actual, (Bytes::from_static(&[0x12, 0x34]), 17));
        }
        assert_eq!(mapped.get(), CALLS);
    }
    for future in futures.drain(..) {
        drop(future);
    }
    assert_eq!(
        LIVE_BYTES.with(Cell::get),
        before,
        "completed or cancelled calls must free all storage"
    );
    assert_eq!(gate.active.load(Ordering::SeqCst), 0);
    assert_eq!(gate.dropped.load(Ordering::SeqCst), CALLS);
    assert_eq!(gate.calls.load(Ordering::SeqCst), CALLS);
    assert_eq!(mapped.get(), if cancel { 0 } else { CALLS });
    gate.ready.store(true, Ordering::SeqCst);
    let mut followup = Box::pin(call(&gate).into_future());
    let Poll::Ready(Ok(actual)) = followup.as_mut().poll(&mut context) else {
        panic!("follow-up must complete after cancellation or success")
    };
    assert_eq!(actual, Bytes::from_static(&[0x12, 0x34]));
    drop(followup);
    assert_eq!(gate.calls.load(Ordering::SeqCst), CALLS + 1);
    assert_eq!(gate.active.load(Ordering::SeqCst), 0);
    assert_eq!(gate.dropped.load(Ordering::SeqCst), CALLS + 1);
    eprintln!("{method} retained {retained} bytes for {CALLS} pending calls");
    assert!(retained <= (CALLS * 256) as isize, "initialization storage remains live: {retained}");
}

#[test]
fn pending_call_storage_complete() {
    pending_storage("eth_call", false);
}

#[test]
fn pending_call_storage_cancel() {
    pending_storage("eth_call", true);
}

#[test]
fn pending_estimate_storage_complete() {
    pending_storage("eth_estimateGas", false);
}

#[test]
fn pending_estimate_storage_cancel() {
    pending_storage("eth_estimateGas", true);
}

#[test]
fn dropping_unpolled_calls_never_starts_the_caller() {
    for method in ["eth_call", "eth_estimateGas"] {
        let gate = Gate::new(method);
        let future = call(&gate).into_future();
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
        drop(future);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
        assert_eq!(gate.active.load(Ordering::SeqCst), 0);
        assert_eq!(Arc::strong_count(&gate), 1);
    }
}

#[test]
fn allocation_accounting_includes_resize_and_release() {
    let before = LIVE_BYTES.with(Cell::get);
    let mut bytes = std::hint::black_box(vec![0_u8; 8]);
    assert_eq!(LIVE_BYTES.with(Cell::get) - before, 8);
    std::hint::black_box(&mut bytes).reserve_exact(24);
    assert_eq!(LIVE_BYTES.with(Cell::get) - before, bytes.capacity() as isize);
    drop(bytes);
    assert_eq!(LIVE_BYTES.with(Cell::get), before);
}

#[derive(Debug)]
struct FailingCaller;

impl Caller<Ethereum, Bytes> for FailingCaller {
    fn call(
        &self,
        _params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        Err(TransportErrorKind::custom_str("caller preparation failed"))
    }

    fn estimate_gas(
        &self,
        _params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        Ok(ProviderCall::ready(Err(TransportErrorKind::custom_str("caller reply failed"))))
    }

    fn call_many(
        &self,
        _params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Bytes>> {
        panic!("ordinary calls must not become callMany")
    }
}

#[test]
fn preparation_and_reply_failures_preserve_the_error_without_mapping() {
    for (method, expected) in
        [("eth_call", "caller preparation failed"), ("eth_estimateGas", "caller reply failed")]
    {
        let mut future = Box::pin(
            EthCall::new(FailingCaller, method, request())
                .map_resp(|_| panic!("errors must not invoke the response mapper"))
                .into_future(),
        );
        let mut context = Context::from_waker(futures::task::noop_waker_ref());
        let Poll::Ready(Err(error)) = future.as_mut().poll(&mut context) else {
            panic!("original error must be returned immediately")
        };
        assert_eq!(error.to_string(), expected);
    }
}

#[derive(Debug)]
enum ReplySource {
    Ready,
    Waiter,
    Rpc(RpcClient),
}

#[derive(Debug)]
struct AlternateCaller {
    source: ReplySource,
    method: &'static str,
}

impl AlternateCaller {
    fn reply(
        &self,
        params: EthCallParams<Ethereum>,
        method: &'static str,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        assert_eq!(method, self.method);
        assert_eq!(params.data(), &request());
        assert_eq!(params.block(), Some(block()));
        let expected = Bytes::from_static(&[0x12, 0x34]);
        Ok(match &self.source {
            ReplySource::Ready => ProviderCall::ready(Ok(expected)),
            ReplySource::Waiter => {
                let (sender, receiver) = oneshot::channel();
                sender.send(Ok(serde_json::value::to_raw_value(&expected).unwrap())).unwrap();
                receiver.into()
            }
            ReplySource::Rpc(client) => client.request(method, params).into(),
        })
    }
}

impl Caller<Ethereum, Bytes> for AlternateCaller {
    fn call(
        &self,
        params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        self.reply(params, "eth_call")
    }

    fn estimate_gas(
        &self,
        params: EthCallParams<Ethereum>,
    ) -> TransportResult<ProviderCall<EthCallParams<Ethereum>, Bytes>> {
        self.reply(params, "eth_estimateGas")
    }

    fn call_many(
        &self,
        _params: EthCallManyParams<'_>,
    ) -> TransportResult<ProviderCall<EthCallManyParams<'static>, Bytes>> {
        panic!("ordinary calls must not become callMany")
    }
}

#[tokio::test]
async fn ready_waiter_and_rpc_replies_keep_method_params_and_mapping() {
    for method in ["eth_call", "eth_estimateGas"] {
        let asserter = Asserter::new();
        asserter.push_success(&Bytes::from_static(&[0x12, 0x34]));
        for source in [
            ReplySource::Ready,
            ReplySource::Waiter,
            ReplySource::Rpc(RpcClient::mocked(asserter.clone())),
        ] {
            let result = EthCall::new(AlternateCaller { source, method }, method, request())
                .block(block())
                .map_resp(|bytes| (bytes, 17))
                .await
                .unwrap();
            assert_eq!(result, (Bytes::from_static(&[0x12, 0x34]), 17));
        }
        assert!(asserter.read_q().is_empty());
    }
}
