use alloy_json_rpc::PubSubItem;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde_json::value::RawValue;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, trace};

type TungsteniteStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// An ongoing connection to a backend.
///
/// This is intended to be spawned in a background task.
pub struct WsBackend<T> {
    pub(crate) socket: T,

    /// Inbound channel from frontend.
    pub(crate) from_frontend: mpsc::UnboundedReceiver<Box<RawValue>>,

    /// Channel of responses to the frontend
    pub(crate) to_frontend: mpsc::UnboundedSender<PubSubItem>,

    /// Notifies the frontend of a terminal error.
    pub(crate) error: oneshot::Sender<()>,

    /// Causes local shutdown when sender is triggered or dropped.
    pub(crate) shutdown: oneshot::Receiver<()>,
}

impl<T> WsBackend<T> {
    pub async fn handle_text(&mut self, t: String) -> Result<(), ()> {
        trace!(text = t, "Received message");

        match serde_json::from_str(&t) {
            Ok(item) => {
                trace!(?item, "Deserialized message");
                let res = self.to_frontend.send(item);
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

    pub async fn handle(&mut self, msg: Message) -> Result<(), ()> {
        match msg {
            Message::Text(t) => self.handle_text(t).await,
            Message::Close(frame) => {
                if frame.is_some() {
                    error!(?frame, "Received close frame with data");
                } else {
                    error!("WS server has gone away");
                }
                Err(())
            }
            Message::Binary(_) => {
                error!("Received binary message, expected text");
                Err(())
            }
            Message::Ping(_) => Ok(()),
            Message::Pong(_) => Ok(()),
            Message::Frame(_) => Ok(()),
        }
    }
}

impl WsBackend<TungsteniteStream> {
    pub fn spawn(mut self) -> JoinHandle<()> {
        let mut err = false;
        tokio::spawn(async move {
            loop {
                #[cfg(not(target_arch = "wasm32"))]
                let keepalive = tokio::time::sleep(std::time::Duration::from_secs(10)).fuse();
                #[cfg(not(target_arch = "wasm32"))]
                tokio::pin!(keepalive);

                tokio::select! {
                    _ = keepalive => {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Err(e) = self.socket.send(Message::Ping(vec![])).await {
                            error!(err = %e, "WS connection error");
                            err = true;
                            break
                        }
                        #[cfg(target_arch = "wasm32")]
                        unreachable!();
                    }
                    resp = self.socket.next() => {
                        match resp {
                            Some(Ok(item)) => {
                                err = self.handle(item).await.is_err();
                                if err { break }
                            },
                            Some(Err(e)) => {
                                error!(err = %e, "WS connection error");
                                err = true;
                                break
                            }
                            None => {
                                error!("WS server has gone away");
                                err = true;
                                break
                            },
                        }
                    }
                    // we've received a new dispatch, so we send it via
                    // websocket
                    inst = self.from_frontend.recv() => {
                        match inst {
                            Some(msg) => {
                                if let Err(e) = self.socket.send(Message::Text(msg.to_string())).await {
                                    error!(err = %e, "WS connection error");
                                    err = true;
                                    break
                                }
                            },
                            // dispatcher has gone away
                            None => {
                                break
                            },
                        }
                    },
                    // break on shutdown recv, or on shutdown recv error
                    _ = &mut self.shutdown => {
                        break
                    },
                }
            }
            if err {
                let _ = self.error.send(());
            }
        })
    }
}
