//! Module for housing transport layers.

mod retry;

/// RetryBackoffLayer
pub use retry::{RateLimitRetryPolicy, RetryBackoffLayer, RetryBackoffService, RetryPolicy};

mod fallback;

/// FallbackLayer
pub use fallback::{FallbackLayer, FallbackService};
