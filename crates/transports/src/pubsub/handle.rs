use alloy_json_rpc::PubSubItem;
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
/// A handle to a backend.
///
/// The backend SHOULD shut down when the handle is dropped (as indicated by
/// the shutdown channel).
pub struct ConnectionHandle {
    /// Outbound channel to server.
    pub(crate) to_socket: mpsc::UnboundedSender<Box<RawValue>>,

    /// Inbound channel from remote server via WS.
    pub(crate) from_socket: mpsc::UnboundedReceiver<PubSubItem>,

    /// Notification from the backend of a terminal error.
    pub(crate) error: oneshot::Receiver<()>,

    /// Notify the backend of intentional shutdown.
    pub(crate) shutdown: oneshot::Sender<()>,
}

impl ConnectionHandle {
    /// Create a new connection handle.
    pub fn new(
        to_socket: mpsc::UnboundedSender<Box<RawValue>>,
        from_socket: mpsc::UnboundedReceiver<PubSubItem>,
        error: oneshot::Receiver<()>,
        shutdown: oneshot::Sender<()>,
    ) -> Self {
        Self {
            to_socket,
            from_socket,
            error,
            shutdown,
        }
    }

    /// Shutdown the backend.
    pub fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
}
