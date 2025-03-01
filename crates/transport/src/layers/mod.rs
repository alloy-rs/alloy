//! Module for housing transport layers.

mod retry;
#[cfg(feature = "throttle")]
mod throttle;

/// RetryBackoffLayer
pub use retry::{RateLimitRetryPolicy, RetryBackoffLayer, RetryBackoffService, RetryPolicy};

#[cfg(feature = "throttle")]
/// ThrottleLayer
pub use throttle::{ThrottleLayer, ThrottleService};
mod fallback;

/// FallbackLayer
pub use fallback::{FallbackLayer, FallbackService};
