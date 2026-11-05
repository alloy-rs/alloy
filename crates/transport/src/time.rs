//! Cross platform time types.
//!
//! On `wasm32-unknown-unknown` this provides minimal replacements for the `tokio::time` utilities
//! used across alloy, backed by `setTimeout` (via [`gloo_timers`]) and [`web_time::Instant`].

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) use std::time::Instant;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use wasm::*;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod wasm {
    use gloo_timers::future::TimeoutFuture;
    use std::{
        future::{Future, IntoFuture},
        pin::Pin,
        task::{ready, Context, Poll},
        time::Duration,
    };

    pub use web_time::Instant;

    /// Maximum delay accepted by `setTimeout`, larger values fire immediately.
    const MAX_TIMEOUT_MS: u128 = i32::MAX as u128;

    /// Returns `now + duration`, saturating to a far future instant on overflow.
    fn deadline_after(duration: Duration) -> Instant {
        let now = Instant::now();
        now.checked_add(duration).unwrap_or_else(|| now + Duration::from_secs(86400 * 365 * 30))
    }

    /// Future returned by [`sleep`] and [`sleep_until`].
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Sleep {
        deadline: Instant,
        timer: Option<TimeoutFuture>,
    }

    impl Sleep {
        /// Returns the instant at which the future will complete.
        pub const fn deadline(&self) -> Instant {
            self.deadline
        }

        /// Resets the future to complete at the new deadline.
        pub fn reset(&mut self, deadline: Instant) {
            self.deadline = deadline;
            self.timer = None;
        }
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            loop {
                let remaining = self.deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    self.timer = None;
                    return Poll::Ready(());
                }
                let timer = self.timer.get_or_insert_with(|| {
                    // round up so that we never wake up before the deadline
                    let millis = remaining.as_nanos().div_ceil(1_000_000).min(MAX_TIMEOUT_MS);
                    TimeoutFuture::new(millis as u32)
                });
                ready!(Pin::new(timer).poll(cx));
                // the timer fired, re-check the deadline in case it was clamped
                self.timer = None;
            }
        }
    }

    /// Waits until `duration` has elapsed.
    pub fn sleep(duration: Duration) -> Sleep {
        sleep_until(deadline_after(duration))
    }

    /// Waits until `deadline` is reached.
    pub const fn sleep_until(deadline: Instant) -> Sleep {
        Sleep { deadline, timer: None }
    }

    /// Error returned by [`Timeout`] when the deadline has elapsed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
    #[error("deadline has elapsed")]
    pub struct Elapsed(());

    /// Future returned by [`timeout`].
    #[derive(Debug)]
    #[pin_project::pin_project]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Timeout<F> {
        #[pin]
        future: F,
        delay: Sleep,
    }

    impl<F> Timeout<F> {
        /// Returns a reference to the inner future.
        pub const fn get_ref(&self) -> &F {
            &self.future
        }

        /// Returns a mutable reference to the inner future.
        pub const fn get_mut(&mut self) -> &mut F {
            &mut self.future
        }

        /// Consumes the timeout, returning the inner future.
        pub fn into_inner(self) -> F {
            self.future
        }
    }

    impl<F: Future> Future for Timeout<F> {
        type Output = Result<F::Output, Elapsed>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            if let Poll::Ready(output) = this.future.poll(cx) {
                return Poll::Ready(Ok(output));
            }
            Pin::new(this.delay).poll(cx).map(|()| Err(Elapsed(())))
        }
    }

    /// Requires `future` to complete before `duration` has elapsed.
    pub fn timeout<F: IntoFuture>(duration: Duration, future: F) -> Timeout<F::IntoFuture> {
        Timeout { future: future.into_future(), delay: sleep(duration) }
    }

    /// Interval returned by [`interval`] and [`interval_at`].
    ///
    /// Missed ticks are fired as fast as possible until caught up, like tokio's default
    /// `MissedTickBehavior::Burst`.
    #[derive(Debug)]
    pub struct Interval {
        delay: Sleep,
        period: Duration,
    }

    impl Interval {
        /// Completes when the next instant in the interval has been reached.
        pub async fn tick(&mut self) -> Instant {
            std::future::poll_fn(|cx| self.poll_tick(cx)).await
        }

        /// Polls for the next instant in the interval to be reached.
        pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Instant> {
            ready!(Pin::new(&mut self.delay).poll(cx));
            let deadline = self.delay.deadline();
            let next =
                deadline.checked_add(self.period).unwrap_or_else(|| deadline_after(self.period));
            self.delay.reset(next);
            Poll::Ready(deadline)
        }

        /// Returns the period of the interval.
        pub const fn period(&self) -> Duration {
            self.period
        }
    }

    /// Creates a new [`Interval`] that yields with interval of `period`. The first tick completes
    /// immediately.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    pub fn interval(period: Duration) -> Interval {
        interval_at(Instant::now(), period)
    }

    /// Creates a new [`Interval`] that yields with interval of `period`, with the first tick
    /// completing at `start`.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    pub fn interval_at(start: Instant, period: Duration) -> Interval {
        assert!(!period.is_zero(), "`period` must be non-zero.");
        Interval { delay: sleep_until(start), period }
    }
}
