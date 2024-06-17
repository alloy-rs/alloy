//! Extended APIs for the provider module.

#[cfg(feature = "admin-api")]
mod admin;
#[cfg(feature = "admin-api")]
pub use admin::AdminApi;

#[cfg(feature = "anvil-api")]
mod anvil;
#[cfg(feature = "anvil-api")]
pub use anvil::AnvilApi;

#[cfg(feature = "engine-api")]
mod engine;
#[cfg(feature = "engine-api")]
pub use engine::EngineApi;

#[cfg(feature = "debug-api")]
mod debug;
#[cfg(feature = "debug-api")]
pub use debug::DebugApi;

#[cfg(feature = "trace-api")]
mod trace;
#[cfg(feature = "trace-api")]
pub use trace::{TraceApi, TraceCallList};

#[cfg(feature = "txpool-api")]
mod txpool;
#[cfg(feature = "txpool-api")]
pub use txpool::TxPoolApi;
