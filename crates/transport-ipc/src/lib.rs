#![doc = include_str!("../README.md")]
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

mod connect;
pub use connect::IpcConnect;

use std::task::Poll::{Pending, Ready};

use alloy_json_rpc::PubSubItem;
use bytes::{Buf, BytesMut};
use futures::{ready, AsyncRead, AsyncWriteExt, StreamExt};
use interprocess::local_socket::{
    tokio::{LocalSocketStream, OwnedReadHalf},
    ToLocalSocketName,
};
use tokio::select;

type Result<T> = std::result::Result<T, std::io::Error>;

/// An IPC backend task.
struct IpcBackend {
    pub(crate) socket: LocalSocketStream,

    pub(crate) interface: alloy_pubsub::ConnectionInterface,
}

impl IpcBackend {
    /// Connect to a local socket. Either a unix socket or a windows named pipe.
    async fn connect<'a, I>(name: &I) -> Result<alloy_pubsub::ConnectionHandle>
    where
        // TODO: remove bound on next interprocess crate release
        I: ToLocalSocketName<'a> + Clone,
    {
        let socket = LocalSocketStream::connect(name.clone()).await?;
        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();

        let backend = IpcBackend { socket, interface };

        backend.spawn();

        Ok(handle)
    }

    fn spawn(mut self) {
        let fut = async move {
            let (read, mut writer) = self.socket.into_split();
            let mut read = ReadJsonStream::new(read).fuse();

            let err = loop {
                select! {
                    biased;
                    item = self.interface.recv_from_frontend() => {
                        match item {
                            Some(msg) => {
                                let bytes = msg.get();
                                if let Err(e) = writer.write_all(bytes.as_bytes()).await {
                                    tracing::error!(%e, "Failed to write to IPC socket");
                                    break true;
                                }
                            },
                            // dispatcher has gone away, or shutdown was received
                            None => {
                                tracing::debug!("Frontend has gone away");
                                break false;
                            },
                        }
                    }
                    // Read from the socket.
                    item = read.next() => {
                        match item {
                            Some(item) => {
                                if self.interface.send_to_frontend(item).is_err() {
                                    tracing::debug!("Frontend has gone away");
                                    break false;
                                }
                            }
                            None => {
                                tracing::error!("Read stream has failed.");
                                break true;
                            }
                        }
                    }
                }
            };
            if err {
                self.interface.close_with_error();
            }
        };

        tokio::spawn(fut);
    }
}

#[pin_project::pin_project]
struct ReadJsonStream {
    #[pin]
    reader: OwnedReadHalf,
    buf: BytesMut,
    items: Vec<PubSubItem>,
}

impl ReadJsonStream {
    fn new(reader: OwnedReadHalf) -> Self {
        Self { reader, buf: BytesMut::with_capacity(4096), items: vec![] }
    }
}

impl futures::stream::Stream for ReadJsonStream {
    type Item = alloy_json_rpc::PubSubItem;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        // Deserialize any buffered items.
        if !this.buf.is_empty() {
            let mut de = serde_json::Deserializer::from_slice(this.buf.as_ref()).into_iter();

            let item = de.next();
            match item {
                Some(Ok(response)) => {
                    this.items.push(response);
                }
                Some(Err(e)) => {
                    tracing::error!(%e, "IPC response contained invalid JSON");
                    return Ready(None);
                }
                None => {}
            }
            this.buf.advance(de.byte_offset());
        }

        if !this.items.is_empty() {
            // may have more work!
            cx.waker().wake_by_ref();
            return Ready(this.items.pop());
        }

        let data = ready!(this.reader.poll_read(cx, this.buf));
        match data {
            Ok(0) => {
                tracing::debug!("IPC socket closed");
                return Ready(None);
            }
            Err(e) => {
                tracing::error!(%e, "Failed to read from IPC socket");
                return Ready(None);
            }
            _ => {
                // wake task to run deserialization
                cx.waker().wake_by_ref();
            }
        }

        Pending
    }
}
