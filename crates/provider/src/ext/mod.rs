//! Extended APIs for the provider module.

#[cfg(feature = "admin-api")]
mod admin;
#[cfg(feature = "admin-api")]
pub use admin::AdminApi;

#[cfg(feature = "anvil-api")]
mod anvil;
#[cfg(feature = "anvil-api")]
pub use anvil::{AnvilApi, ImpersonateConfig};

#[cfg(feature = "engine-api")]
mod engine;
#[cfg(feature = "engine-api")]
pub use engine::EngineApi;

#[cfg(feature = "engine-api")]
mod testing;
#[cfg(feature = "engine-api")]
pub use testing::TestingApi;

#[cfg(feature = "debug-api")]
mod debug;
#[cfg(feature = "debug-api")]
pub use debug::DebugApi;

#[cfg(feature = "net-api")]
mod net;
#[cfg(feature = "net-api")]
pub use net::NetApi;

#[cfg(feature = "trace-api")]
mod trace;
#[cfg(feature = "trace-api")]
pub use trace::{TraceApi, TraceBuilder, TraceCallList, TraceParams};

#[cfg(feature = "rpc-api")]
mod rpc;
#[cfg(feature = "rpc-api")]
pub use rpc::RpcApi;

#[cfg(feature = "txpool-api")]
mod txpool;
#[cfg(feature = "txpool-api")]
pub use txpool::TxPoolApi;

#[cfg(feature = "erc4337-api")]
mod erc4337;
#[cfg(feature = "erc4337-api")]
pub use erc4337::Erc4337Api;

#[cfg(feature = "tenderly-api")]
mod tenderly;
#[cfg(feature = "tenderly-api")]
pub use tenderly::TenderlyApi;

#[cfg(feature = "tenderly-admin-api")]
mod tenderly_admin;
#[cfg(feature = "tenderly-admin-api")]
pub use tenderly_admin::TenderlyAdminApi;

#[cfg(feature = "mev-api")]
mod mev;

#[cfg(feature = "mev-api")]
pub use mev::{
    sign_flashbots_payload, verify_flashbots_signature, FlashbotsSignatureError, MevApi,
    MevBuilder, FLASHBOTS_SIGNATURE_HEADER,
};

/// Reth related apis.
pub mod reth;

#[cfg(test)]
pub(crate) mod test {
    #[allow(dead_code)] // dead only when all features off
    /// Run the given function only if we are in a CI environment.
    pub(crate) async fn async_ci_only<F, Fut>(f: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        if ci_info::is_ci() {
            f().await;
        }
    }
}
