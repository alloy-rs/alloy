// #![doc = include_str!("../README.md")]
// #![doc(
//     html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
//     html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
// )]
// #![warn(
//     missing_copy_implementations,
//     missing_debug_implementations,
//     missing_docs,
//     unreachable_pub,
//     clippy::missing_const_for_fn,
//     rustdoc::all
// )]
// #![cfg_attr(not(test), warn(unused_crate_dependencies))]
// #![deny(unused_must_use, rust_2018_idioms)]
// #![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

use alloy_pubsub::ConnectionInterface;

/// IPC Connection details
pub struct IpcConnect {
    path: std::path::PathBuf,
}

/// An ongoing IPC connection to a backend.
pub struct IpcBackend<T> {
    /// The IPC socket connection. For windows this is a named pipe, for unix
    /// it is a unix socket.
    pub(crate) stream: T,

    /// The interface to the connection.
    pub(crate) interface: ConnectionInterface,
}
