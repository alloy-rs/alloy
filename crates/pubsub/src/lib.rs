#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

mod connect;
pub use connect::PubSubConnect;

mod frontend;
pub use frontend::PubSubFrontend;

mod ix;
pub use ix::PubSubInstruction;

mod handle;
pub use handle::{ConnectionHandle, ConnectionInterface};

mod managers;
pub use managers::InFlight;

mod service;

mod sub;
pub use sub::{
    RawSubscription, SubAnyStream, SubResultStream, Subscription, SubscriptionItem,
    SubscriptionStream,
};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use getrandom as _; // Enable rand's browser entropy source.

mod recovery;
pub use recovery::{with_request_deadline, PartialBatchError, RecoveryBackoff};

// Preserve the transport's native and browser clock implementations.
mod time {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub(crate) use tokio::time::{sleep_until, timeout_at, Instant};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub(crate) use wasmtimer::{
        std::Instant,
        tokio::{sleep_until, timeout_at},
    };
}
