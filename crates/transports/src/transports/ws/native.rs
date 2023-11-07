use crate::{pubsub::PubSubConnect, utils::Spawnable, TransportError};

use futures::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::{future::Future, pin::Pin, time::Duration};
use tokio::time::sleep;
use tokio_tungstenite::{
    tungstenite::{self, client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};
use tracing::error;

use super::WsBackend;

type TungsteniteStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const KEEPALIVE: u64 = 10;

impl WsBackend<TungsteniteStream> {
    pub async fn handle(&mut self, msg: Message) -> Result<(), ()> {
        match msg {
            Message::Text(text) => self.handle_text(text).await,
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

    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), tungstenite::Error> {
        self.socket.send(Message::Text(msg.get().to_owned())).await
    }

    /// Spawn a new backend task.
    pub fn spawn(mut self) {
        let fut = async move {
            let mut err = false;
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
        };
        fut.spawn_task()
    }
}

#[derive(Debug, Clone)]
pub struct WsConnect {
    pub url: String,
    pub auth: Option<crate::Authorization>,
}

impl IntoClientRequest for WsConnect {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let mut request: http::Request<()> = self.url.into_client_request()?;
        if let Some(auth) = self.auth {
            let mut auth_value = http::HeaderValue::from_str(&auth.to_string())?;
            auth_value.set_sensitive(true);

            request
                .headers_mut()
                .insert(http::header::AUTHORIZATION, auth_value);
        }

        request.into_client_request()
    }
}

impl PubSubConnect for WsConnect {
    fn is_local(&self) -> bool {
        crate::utils::guess_local_url(&self.url)
    }

    fn connect<'a: 'b, 'b>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<crate::pubsub::ConnectionHandle, TransportError>>
                + Send
                + 'b,
        >,
    > {
        let request = self.clone().into_client_request();

        Box::pin(async move {
            let (socket, _) = tokio_tungstenite::connect_async(request?).await?;

            let (handle, interface) = crate::pubsub::ConnectionHandle::new();
            let backend = WsBackend { socket, interface };

            backend.spawn();

            Ok(handle)
        })
    }
}
