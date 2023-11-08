use std::task::Poll;

use alloy_json_rpc::PubSubItem;
use futures::{FutureExt, Stream};
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
/// A handle to a backend. Communicates to a `ConnectionInterface` on the
/// backend.
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
    pub fn new() -> (Self, ConnectionInterface) {
        let (to_socket, from_frontend) = mpsc::unbounded_channel();
        let (to_frontend, from_socket) = mpsc::unbounded_channel();
        let (error_tx, error_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let handle = Self {
            to_socket,
            from_socket,
            error: error_rx,
            shutdown: shutdown_tx,
        };
        let interface = ConnectionInterface {
            from_frontend,
            to_frontend,
            error: error_tx,
            shutdown: shutdown_rx,
            dead: false,
        };
        (handle, interface)
    }

    /// Shutdown the backend.
    pub fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
}

/// The reciprocal of [`ConnectionHandle`].
///
/// [`ConnectionInterface`] implements [`Stream`] for receiving requests from
/// the frontend. The [`Stream`] implementation will return `None` permanently
/// when the shutdown channel from the frontend has resolved.
///
///  It sends responses to the frontend via the `send_to_frontend`
/// method. It also notifies the frontend of a terminal error via the `error`
/// channel.

pub struct ConnectionInterface {
    /// Inbound channel from frontend.
    pub(crate) from_frontend: mpsc::UnboundedReceiver<Box<RawValue>>,

    /// Channel of responses to the frontend
    pub(crate) to_frontend: mpsc::UnboundedSender<PubSubItem>,

    /// Notifies the frontend of a terminal error.
    pub(crate) error: oneshot::Sender<()>,

    /// Causes local shutdown when sender is triggered or dropped.
    pub(crate) shutdown: oneshot::Receiver<()>,

    /// True when the shutdown command has been received
    dead: bool,
}

impl ConnectionInterface {
    /// Send a pubsub item to the frontend.
    pub fn send_to_frontend(
        &self,
        item: PubSubItem,
    ) -> Result<(), mpsc::error::SendError<PubSubItem>> {
        self.to_frontend.send(item)
    }

    /// Receive a request from the frontend.
    pub async fn recv_from_frontend(&mut self) -> Option<Box<RawValue>> {
        self.from_frontend.recv().await
    }

    /// Close the interface, sending an error to the frontend.
    pub fn close_with_error(self) {
        let _ = self.error.send(());
    }
}

impl Stream for ConnectionInterface {
    type Item = Box<RawValue>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.dead {
            return Poll::Ready(None);
        }

        if let Poll::Ready(_) = self.shutdown.poll_unpin(cx) {
            self.dead = true;
            return Poll::Ready(None);
        }

        self.from_frontend.poll_recv(cx)
    }
}
