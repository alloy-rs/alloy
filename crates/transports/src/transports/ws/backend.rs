use futures_util::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::{task::JoinHandle, time::sleep};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, trace};

use crate::pubsub::ConnectionInterface;

type TungsteniteStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const KEEPALIVE: u64 = 10;

/// An ongoing connection to a backend.
///
/// Users should NEVER instantiate a backend directly. Instead, they should use
/// [`PubSubConnect`] to get a running service with a running backend.
///
/// [`PubSubConnect`]: crate::pubsub::PubSubConnect
pub(crate) struct WsBackend<T> {
    pub(crate) socket: T,

    pub(crate) interface: ConnectionInterface,
}

impl<T> WsBackend<T> {
    pub async fn handle_text(&mut self, t: String) -> Result<(), ()> {
        trace!(text = t, "Received message");

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

impl WsBackend<TungsteniteStream> {
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

    pub async fn send(
        &mut self,
        msg: Box<RawValue>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        self.socket.send(Message::Text(msg.to_string())).await
    }

    /// Spawn a new backend task.
    pub fn spawn(mut self) -> JoinHandle<()> {
        let mut err = false;
        tokio::spawn(async move {
            let keepalive = sleep(Duration::from_secs(KEEPALIVE));
            tokio::pin!(keepalive);
            loop {
                // We bias the loop as follows
                // 1. Shutdown channels.
                // 2. New dispatch to server.
                // 3. Keepalive.
                // 4. Response or notification from server.
                // This ensures that keepalive is sent only if no other messages
                // have been sent in the last 10 seconds. And prioritizes new
                // dispatches over responses from the server. This will fail if
                // the client saturates the task with dispatches, but that's
                // probably not a big deal.
                tokio::select! {
                    biased;
                    // break on shutdown recv, or on shutdown recv error
                    _ = &mut self.interface.shutdown => {
                        self.interface.from_frontend.close();
                        break
                    },
                    // we've received a new dispatch, so we send it via
                    // websocket. We handle new work before processing any
                    // responses from the server.
                    inst = self.interface.from_frontend.recv() => {
                        match inst {
                            Some(msg) => {
                                // Reset the keepalive timer.
                                keepalive.set(sleep(Duration::from_secs(KEEPALIVE)));
                                if let Err(e) = self.send(msg).await {
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
                    // Send a ping to the server, if no other messages have been
                    // sent in the last 10 seconds.
                    _ = &mut keepalive => {
                        // Reset the keepalive timer.
                        keepalive.set(sleep(Duration::from_secs(KEEPALIVE)));
                        if let Err(e) = self.socket.send(Message::Ping(vec![])).await {
                            error!(err = %e, "WS connection error");
                            err = true;
                            break
                        }
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
                }
            }
            if err {
                let _ = self.interface.error.send(());
            }
        })
    }
}
