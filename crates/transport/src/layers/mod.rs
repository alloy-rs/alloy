//! Module for housing transport layers.

/// ThrottleLayer
#[cfg(feature = "throttle")]
mod throttle;
#[cfg(feature = "throttle")]
pub use throttle::{ThrottleLayer, ThrottleService};

/// RetryBackoffLayer
mod retry;
pub use retry::{
    OrRetryPolicyFn, RateLimitRetryPolicy, RetryBackoffLayer, RetryBackoffService, RetryPolicy,
};

/// FallbackLayer
mod fallback;
pub use fallback::{FallbackLayer, FallbackService};

pub mod redundancy;
pub use redundancy::{RedundancyLayer, RedundancyService};
