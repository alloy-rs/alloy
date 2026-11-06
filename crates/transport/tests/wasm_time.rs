//! Regression tests using the actual JavaScript timer backend.

#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use alloy_transport::time::{interval, interval_at, sleep, sleep_until, timeout, Instant};
use futures::{
    future::{pending, ready},
    FutureExt,
};
use std::time::Duration;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn timers_and_timeouts_work_on_the_js_event_loop() {
    let start = Instant::now();
    sleep(Duration::from_millis(5)).await;
    assert!(start.elapsed() >= Duration::from_millis(5));
    sleep_until(start).await;
    assert_eq!(timeout(Duration::ZERO, ready(42)).await.unwrap(), 42);
    assert!(timeout(Duration::from_millis(5), pending::<()>()).await.is_err());
    // Pin projection must support futures that are not Unpin.
    assert_eq!(timeout(Duration::from_secs(1), async { 7 }).await.unwrap(), 7);
}

#[wasm_bindgen_test]
async fn intervals_preserve_first_tick_and_schedule() {
    let period = Duration::from_millis(5);
    let mut immediate = interval(period);
    assert!(immediate.tick().now_or_never().is_some());
    let start = Instant::now() + period;
    let mut delayed = interval_at(start, period);
    assert!(delayed.tick().now_or_never().is_none());
    assert_eq!(delayed.tick().await, start);
    assert_eq!(delayed.tick().await, start + period);
}

#[wasm_bindgen_test]
fn dropping_a_long_sleep_cancels_the_timer() {
    // This should neither wrap the delay nor keep the Node test runner alive.
    drop(sleep(Duration::from_secs(60 * 60 * 24 * 365)));
}
