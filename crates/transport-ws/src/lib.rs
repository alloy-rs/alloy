#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[macro_use]
extern crate tracing;

use alloy_pubsub::ConnectionInterface;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::WsConnect;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::WsConnect;

/// An ongoing connection to a backend.
///
/// Users should NEVER instantiate a backend directly. Instead, they should use
/// [`PubSubConnect`] to get a running service with a running backend.
///
/// [`PubSubConnect`]: alloy_pubsub::PubSubConnect
#[derive(Debug)]
pub struct WsBackend<T> {
    /// The websocket connection.
    pub(crate) socket: T,

    /// The interface to the connection.
    pub(crate) interface: ConnectionInterface,
}

impl<T> WsBackend<T> {
    /// Handle inbound text from the websocket.
    #[allow(clippy::result_unit_err)]
    pub fn handle_text(&mut self, text: &str) -> Result<(), ()> {
        trace!(%text, "received message from websocket");

        match serde_json::from_str(text) {
            Ok(item) => {
                trace!(?item, "deserialized message");
                if let Err(err) = self.interface.send_to_frontend(item) {
                    error!(item=?err.0, "failed to send deserialized item to handler");
                    return Err(());
                }
            }
            Err(err) => {
                error!(%err, "failed to deserialize message");
                return Err(());
            }
        }
        Ok(())
    }
}
