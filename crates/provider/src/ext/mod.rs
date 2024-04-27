//! Extended APIs for the provider module.

mod admin;
pub use admin::AdminApi;

mod debug;
pub use debug::DebugApi;

mod txpool;
pub use txpool::TxPoolApi;
