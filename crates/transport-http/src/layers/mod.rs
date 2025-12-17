//! Additional layers that may be useful for HTTP transport users. Typically
//! these inject headers into the HTTP requests.
#![cfg(not(target_family = "wasm"))]

#[cfg(feature = "jwt-auth")]
mod auth;
#[cfg(feature = "jwt-auth")]
pub use auth::{AuthLayer, AuthService};

#[cfg(feature = "traceparent")]
mod trace;
#[cfg(feature = "traceparent")]
pub use trace::{TraceParentLayer, TraceParentService};
