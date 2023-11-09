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

//! alloy-transports-ws

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::WsConnect;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::WsConnect;

use alloy_pubsub::ConnectionInterface;

use tracing::{debug, error, trace};

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
    #[tracing::instrument(skip(self))]
    pub async fn handle_text(&mut self, t: String) -> Result<(), ()> {
        debug!(text = t, "Received message from websocket");

        match serde_json::from_str(&t) {
            Ok(item) => {
                trace!(?item, "Deserialized message");
                let res = self.interface.send_to_frontend(item);
                if res.is_err() {
                    error!("Failed to send message to handler");
                    return Err(());
                }
            }
            Err(e) => {
                error!(e = %e, "Failed to deserialize message");
                return Err(());
            }
        }
        Ok(())
    }
}
