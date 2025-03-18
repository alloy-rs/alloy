//! Useful layer implementations for the provider.

#[cfg(any(test, feature = "anvil-node"))]
mod anvil;
#[cfg(any(test, feature = "anvil-node"))]
pub use anvil::{AnvilLayer, AnvilProvider};

mod batch;
pub use batch::{CallBatchLayer, CallBatchProvider};

mod chain;
pub use chain::ChainLayer;

#[cfg(not(target_arch = "wasm32"))]
mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub use cache::{CacheLayer, CacheProvider, SharedCache};
