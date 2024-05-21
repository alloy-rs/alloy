#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[macro_use]
extern crate tracing;

mod batch;
pub use batch::BatchRequest;

mod builder;
pub use builder::ClientBuilder;

mod builtin;
pub use builtin::BuiltInConnectionString;

mod call;
pub use call::RpcCall;

mod client;
pub use client::{ClientRef, RpcClient, WeakClient};

mod poller;
pub use poller::{PollChannel, PollerBuilder};

#[cfg(feature = "ws")]
pub use alloy_transport_ws::WsConnect;

#[cfg(all(feature = "ipc", not(target_arch = "wasm32")))]
pub use alloy_transport_ipc::IpcConnect;
