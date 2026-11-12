//! Browser timer adapters shared by Alloy crates.
//!
//! The public API is available on browser WASM targets. Native call sites use Tokio directly.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) use std::time::Instant;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use browser::*;

// Compile the browser adapter on native test builds as well, so its scheduling
// logic can be tested independently of the JavaScript timer backend.
#[cfg(any(test, all(target_family = "wasm", target_os = "unknown")))]
#[cfg_attr(
    not(all(target_family = "wasm", target_os = "unknown")),
    allow(dead_code, unreachable_pub)
)]
mod browser {
    use futures::{future::poll_fn, Future};
    use futures_timer::Delay;
    use pin_project::pin_project;
    use std::{
        fmt,
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };
    pub use web_time::Instant;

    // JavaScript setTimeout accepts a signed 32-bit millisecond delay. Chunk
    // longer waits and round fractional milliseconds up to avoid early wakeups.
    fn timer_chunk(duration: Duration) -> Duration {
        let millis =
            duration.as_millis() + u128::from(!duration.subsec_nanos().is_multiple_of(1_000_000));
        Duration::from_millis(millis.min(i32::MAX as u128) as u64)
    }

    /// Future returned by [`sleep`] and [`sleep_until`].
    #[derive(Debug)]
    pub struct Sleep {
        deadline: Instant,
        delay: Delay,
    }

    /// Waits for a duration.
    pub fn sleep(duration: Duration) -> Sleep {
        sleep_until(Instant::now() + duration)
    }

    /// Waits until a deadline, completing immediately if it has already passed.
    pub fn sleep_until(deadline: Instant) -> Sleep {
        Sleep {
            deadline,
            delay: Delay::new(timer_chunk(deadline.saturating_duration_since(Instant::now()))),
        }
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let this = self.get_mut();
            loop {
                let remaining = this.deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Poll::Ready(());
                }
                if Pin::new(&mut this.delay).poll(cx).is_pending() {
                    return Poll::Pending;
                }
                this.delay.reset(timer_chunk(remaining));
            }
        }
    }

    /// Error returned when a [`Timeout`] expires.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Elapsed;

    impl fmt::Display for Elapsed {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("deadline has elapsed")
        }
    }
    impl std::error::Error for Elapsed {}

    /// A future with a time limit.
    #[pin_project]
    #[derive(Debug)]
    pub struct Timeout<F> {
        #[pin]
        future: F,
        delay: Sleep,
    }

    /// Limits how long a future may run. A ready future wins over the timer.
    pub fn timeout<F: Future>(duration: Duration, future: F) -> Timeout<F> {
        Timeout { future, delay: sleep(duration) }
    }

    impl<F> Timeout<F> {
        /// Returns a reference to the wrapped future.
        pub const fn get_ref(&self) -> &F {
            &self.future
        }
        /// Returns a mutable reference to the wrapped future.
        pub const fn get_mut(&mut self) -> &mut F {
            &mut self.future
        }
        /// Returns the wrapped future.
        pub fn into_inner(self) -> F {
            self.future
        }
    }

    impl<F: Future> Future for Timeout<F> {
        type Output = Result<F::Output, Elapsed>;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            if let Poll::Ready(value) = this.future.poll(cx) {
                return Poll::Ready(Ok(value));
            }
            Pin::new(this.delay).poll(cx).map(|()| Err(Elapsed))
        }
    }

    /// A fixed-period timer with burst behavior for missed ticks.
    #[derive(Debug)]
    pub struct Interval {
        delay: Sleep,
        period: Duration,
    }

    /// Creates an interval whose first tick completes immediately.
    pub fn interval(period: Duration) -> Interval {
        interval_at(Instant::now(), period)
    }

    /// Creates an interval whose first tick occurs at `start`.
    ///
    /// Panics if `period` is zero.
    pub fn interval_at(start: Instant, period: Duration) -> Interval {
        assert!(!period.is_zero(), "period must be non-zero");
        Interval { delay: sleep_until(start), period }
    }

    impl Interval {
        /// Waits for the next scheduled tick.
        pub async fn tick(&mut self) -> Instant {
            poll_fn(|cx| self.poll_tick(cx)).await
        }

        /// Polls for the next tick without shifting the interval's schedule.
        pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Instant> {
            futures::ready!(Pin::new(&mut self.delay).poll(cx));
            let deadline = self.delay.deadline;
            self.delay = sleep_until(deadline + self.period);
            Poll::Ready(deadline)
        }
    }

    #[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
    mod tests {
        use super::*;
        use futures::{
            future::{pending, ready},
            task::noop_waker_ref,
        };

        #[test]
        fn chunks_round_up_and_do_not_overflow() {
            assert_eq!(timer_chunk(Duration::from_nanos(1)), Duration::from_millis(1));
            assert_eq!(timer_chunk(Duration::MAX), Duration::from_millis(i32::MAX as u64));
            assert_eq!(timer_chunk(Duration::ZERO), Duration::ZERO);
        }

        #[tokio::test]
        async fn timeout_prioritizes_ready_future_and_expires_pending_future() {
            assert_eq!(timeout(Duration::ZERO, ready(42)).await, Ok(42));
            assert_eq!(timeout(Duration::ZERO, pending::<()>()).await, Err(Elapsed));
            assert_eq!(timeout(Duration::from_millis(2), pending::<()>()).await, Err(Elapsed));
        }

        #[tokio::test]
        async fn sleep_observes_deadline() {
            let deadline = Instant::now() + Duration::from_millis(2);
            sleep_until(deadline).await;
            assert!(Instant::now() >= deadline);
            sleep_until(deadline).await;
        }

        #[test]
        fn intervals_keep_the_schedule_and_burst_after_missed_ticks() {
            let start = Instant::now() - Duration::from_secs(3);
            let mut timer = interval_at(start, Duration::from_secs(1));
            let mut cx = Context::from_waker(noop_waker_ref());
            assert_eq!(timer.poll_tick(&mut cx), Poll::Ready(start));
            assert_eq!(timer.poll_tick(&mut cx), Poll::Ready(start + Duration::from_secs(1)));
        }

        #[test]
        fn delayed_first_tick_is_pending() {
            let mut timer =
                interval_at(Instant::now() + Duration::from_secs(60), Duration::from_secs(1));
            let mut cx = Context::from_waker(noop_waker_ref());
            assert!(timer.poll_tick(&mut cx).is_pending());
        }

        #[test]
        #[should_panic(expected = "period must be non-zero")]
        fn zero_period_panics() {
            interval(Duration::ZERO);
        }
    }
}
