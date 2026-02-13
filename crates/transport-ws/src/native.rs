use crate::{WsBackend, DEFAULT_KEEPALIVE};
use alloy_pubsub::PubSubConnect;
use alloy_transport::{utils::Spawnable, Authorization, TransportErrorKind, TransportResult};
use futures::{SinkExt, StreamExt};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::time::sleep;
use yawc::{
    frame::{Frame, OpCode},
    Options, WebSocket,
};

/// Re-export yawc's [`Options`] as the WebSocket configuration type.
pub type WebSocketConfig = Options;

/// Simple connection details for a websocket connection.
#[derive(Clone)]
pub struct WsConnect {
    /// The URL to connect to.
    url: String,
    /// The authorization header to use.
    auth: Option<Authorization>,
    /// The websocket config.
    config: Option<Options>,
    /// Max number of retries before failing and exiting the connection.
    /// Default is 10.
    max_retries: u32,
    /// The interval between retries.
    /// Default is 3 seconds.
    retry_interval: Duration,
    /// The keepalive interval for sending pings.
    /// Default is 10 seconds.
    keepalive_interval: Duration,
}

impl std::fmt::Debug for WsConnect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsConnect")
            .field("url", &self.url)
            .field("auth", &self.auth)
            .field("config", &self.config.as_ref().map(|_| ".."))
            .field("max_retries", &self.max_retries)
            .field("retry_interval", &self.retry_interval)
            .field("keepalive_interval", &self.keepalive_interval)
            .finish()
    }
}

impl WsConnect {
    /// Creates a new websocket connection configuration.
    pub fn new<S: Into<String>>(url: S) -> Self {
        Self {
            url: url.into(),
            auth: None,
            config: None,
            max_retries: 10,
            retry_interval: Duration::from_secs(3),
            keepalive_interval: Duration::from_secs(DEFAULT_KEEPALIVE),
        }
    }

    /// Sets the authorization header.
    pub fn with_auth(mut self, auth: Authorization) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Sets the optional authorization header.
    ///
    /// This replaces the current [`Authorization`].
    pub fn with_auth_opt(mut self, auth: Option<Authorization>) -> Self {
        self.auth = auth;
        self
    }

    /// Sets the websocket config.
    pub const fn with_config(mut self, config: Options) -> Self {
        self.config = Some(config);
        self
    }

    /// Get the URL string of the connection.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the authorization header.
    pub const fn auth(&self) -> Option<&Authorization> {
        self.auth.as_ref()
    }

    /// Get the websocket config.
    pub const fn config(&self) -> Option<&Options> {
        self.config.as_ref()
    }

    /// Sets the max number of retries before failing and exiting the connection.
    /// Default is 10.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the interval between retries.
    /// Default is 3 seconds.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = retry_interval;
        self
    }

    /// Sets the keepalive ping interval.
    ///
    /// A ping is sent if no other messages have been sent within this interval.
    /// If the server does not respond with a pong before the next ping is due,
    /// the connection is considered dead and will be closed.
    ///
    /// Default is 10 seconds.
    pub const fn with_keepalive_interval(mut self, keepalive_interval: Duration) -> Self {
        self.keepalive_interval = keepalive_interval;
        self
    }
}

impl PubSubConnect for WsConnect {
    fn is_local(&self) -> bool {
        alloy_transport::utils::guess_local_url(&self.url)
    }

    async fn connect(&self) -> TransportResult<alloy_pubsub::ConnectionHandle> {
        let url: url::Url = self.url.parse().map_err(TransportErrorKind::custom)?;
        let mut builder = WebSocket::connect(url);

        // always add utf8 support to prevent `frame.as_str()` from panicking
        let options = self.config.clone().unwrap_or_default().with_utf8();
        builder = builder.with_options(options);

        if let Some(auth) = &self.auth {
            builder = builder.with_request(
                yawc::HttpRequestBuilder::default().header("Authorization", auth.to_string()),
            );
        }

        let socket = builder.await.map_err(TransportErrorKind::custom)?;

        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = WsBackend { socket, interface, keepalive_interval: self.keepalive_interval };

        backend.spawn();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
    }
}

impl<S> WsBackend<WebSocket<S>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    /// Handle a message from the server.
    #[expect(clippy::result_unit_err)]
    pub fn handle(&mut self, frame: Frame) -> Result<(), ()> {
        match frame.opcode() {
            OpCode::Text => self.handle_text(frame.as_str()),
            OpCode::Close => {
                if let Ok(Some(reason)) = frame.close_reason() {
                    error!(%reason, "Received close frame with data");
                } else {
                    error!("WS server has gone away");
                }
                Err(())
            }
            OpCode::Binary => {
                error!("Received binary message, expected text");
                Err(())
            }
            OpCode::Ping | OpCode::Pong | OpCode::Continuation => Ok(()),
        }
    }

    /// Send a message to the server.
    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), yawc::WebSocketError> {
        self.socket.send(Frame::text(msg.get().to_owned())).await
    }

    /// Spawn a new backend task.
    pub fn spawn(mut self) {
        let fut = async move {
            let mut errored = false;
            let mut expecting_pong = false;
            let keepalive = sleep(self.keepalive_interval);
            tokio::pin!(keepalive);
            loop {
                // We bias the loop as follows
                // 1. New dispatch to server.
                // 2. Keepalive.
                // 3. Response or notification from server.
                // This ensures that keepalive is sent only if no other messages
                // have been sent in the keepalive interval. And prioritizes new
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
                                keepalive.set(sleep(self.keepalive_interval));
                                if let Err(err) = self.send(msg).await {
                                    error!(%err, "WS connection error");
                                    errored = true;
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
                    // sent within the keepalive interval.
                    _ = &mut keepalive => {
                        // Still expecting a pong from the previous ping,
                        // meaning connection is errored.
                        if expecting_pong {
                            error!("WS server missed a pong");
                            errored = true;
                            break
                        }
                        // Reset the keepalive timer.
                        keepalive.set(sleep(self.keepalive_interval));
                        if let Err(err) = self.socket.send(Frame::ping(&b""[..])).await {
                            error!(%err, "WS connection error");
                            errored = true;
                            break
                        }
                        // Expecting to receive a pong before the next
                        // keepalive timer resolves.
                        expecting_pong = true;
                    }
                    resp = self.socket.next() => {
                        match resp {
                            Some(item) => {
                                if item.opcode() == OpCode::Pong {
                                    expecting_pong = false;
                                }
                                errored = self.handle(item).is_err();
                                if errored { break }
                            },
                            None => {
                                error!("WS server has gone away");
                                errored = true;
                                break
                            },
                        }
                    }
                }
            }
            if errored {
                self.interface.close_with_error();
            }
        };
        fut.spawn_task()
    }
}
