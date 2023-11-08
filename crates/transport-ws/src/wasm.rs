use super::WsBackend;
use alloy_transport::utils::Spawnable;

use futures::{
    sink::SinkExt,
    stream::{Fuse, StreamExt},
};
use serde_json::value::RawValue;
use tracing::error;
use ws_stream_wasm::{WsErr, WsMessage, WsStream};

impl WsBackend<Fuse<WsStream>> {
    pub async fn handle(&mut self, item: WsMessage) -> Result<(), ()> {
        match item {
            WsMessage::Text(text) => self.handle_text(text).await,
            WsMessage::Binary(_) => {
                error!("Received binary message, expected text");
                Err(())
            }
        }
    }

    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), WsErr> {
        self.socket
            .send(WsMessage::Text(msg.get().to_owned()))
            .await
    }

    pub fn spawn(mut self) {
        let fut = async move {
            let mut err = false;
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
                    // we've received a new dispatch, so we send it via
                    // websocket. We handle new work before processing any
                    // responses from the server.
                    inst = self.interface.recv_from_frontend() => {
                        match inst {
                            Some(msg) => {
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
                    resp = self.socket.next() => {
                        match resp {
                            Some(item) => {
                                err = self.handle(item).await.is_err();
                                if err { break }
                            },
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
                self.interface.close_with_error();
            }
        };
        fut.spawn_task();
    }
}

#[derive(Debug, Clone)]
pub struct WsConnect {
    pub url: String,
}
