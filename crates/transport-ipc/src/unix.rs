use crate::{IpcBackend, IpcConnect};

use alloy_pubsub::PubSubConnect;
use alloy_transport::{TransportError, TransportErrorKind};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::{net::UnixStream, time::sleep};
use tracing::error;

const KEEPALIVE: u64 = 10;

impl PubSubConnect for IpcConnect {
    fn is_local(&self) -> bool {
        true
    }

    fn connect<'a: 'b, 'b>(
        &'a self,
    ) -> alloy_transport::Pbf<'b, alloy_pubsub::ConnectionHandle, TransportError> {
        Box::pin(async move {
            let stream =
                UnixStream::connect(&self.path).await.map_err(TransportErrorKind::custom)?;

            let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
            let backend = IpcBackend { stream, interface };

            backend.spawn();

            Ok(handle)
        })
    }
}

impl IpcBackend<UnixStream> {
    /// Handle a message from the server.
    pub async fn handle(&mut self, msg: Box<RawValue>) -> Result<(), ()> {
        todo!()
    }

    /// Send a message to the server.
    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), ()> {
        todo!()
    }

    fn spawn(self) {
        tokio::spawn(async move {
            let mut err = false;
            let keepalive = sleep(Duration::from_secs(KEEPALIVE));

            loop {
                // We bias the loop as follows
                // 1. New dispatch.
                // 2. Keepalive.
                // 3. Response or notification from server.
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
                                // Reset the keepalive timer.
                                keepalive.set(sleep(Duration::from_secs(KEEPALIVE)));
                                if let Err(e) = self.send(msg).await {
                                    error!(err = %e, "WS connection error");
                                    err = true;
                                    break
                                }
                            },
                            // dispatcher has gone away, or shutdown was received
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
                        if let Err(e) = self.stream.send(Message::Ping(vec![])).await {
                            error!(err = %e, "WS connection error");
                            err = true;
                            break
                        }
                    }
                    resp = self.stream.next() => {
                        match resp {
                            Some(Ok(item)) => {
                                err = self.handle(item).await.is_err();
                                if err { break }
                            },
                            Some(Err(e)) => {
                                tracing::error!(err = %e, "WS connection error");
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
                self.interface.close_with_error();
            }
        });
    }
}
