#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::WsConnect;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::WsConnect;

use crate::pubsub::ConnectionInterface;

use tracing::{debug, error, trace};

/// An ongoing connection to a backend.
///
/// Users should NEVER instantiate a backend directly. Instead, they should use
/// [`PubSubConnect`] to get a running service with a running backend.
///
/// [`PubSubConnect`]: crate::PubSubConnect
pub struct WsBackend<T> {
    pub(crate) socket: T,

    pub(crate) interface: ConnectionInterface,
}

impl<T> WsBackend<T> {
    #[tracing::instrument(skip(self))]
    pub async fn handle_text(&mut self, t: String) -> Result<(), ()> {
        debug!(text = t, "Received message from websocket");

        match serde_json::from_str(&t) {
            Ok(item) => {
                trace!(?item, "Deserialized message");
                let res = self.interface.to_frontend.send(item);
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
