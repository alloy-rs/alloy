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

#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "mock")]
pub use mock::MockIpcServer;

use std::task::Poll::{Pending, Ready};

use alloy_json_rpc::PubSubItem;
use bytes::{Buf, BytesMut};
use futures::{io::BufReader, ready, AsyncBufRead, AsyncRead, AsyncWriteExt, StreamExt};
use interprocess::local_socket::{tokio::LocalSocketStream, ToLocalSocketName};
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

/// A stream of JSON-RPC items, read from an [`AsyncRead`] stream.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct ReadJsonStream<T> {
    /// The underlying reader.
    #[pin]
    reader: BufReader<T>,
    /// A buffer of bytes read from the reader.
    buf: BytesMut,
    /// A buffer of items deserialized from the reader.
    items: Vec<PubSubItem>,
}

impl<T> ReadJsonStream<T>
where
    T: AsyncRead,
{
    fn new(reader: T) -> Self {
        Self { reader: BufReader::new(reader), buf: BytesMut::with_capacity(4096), items: vec![] }
    }
}

impl<T> From<T> for ReadJsonStream<T>
where
    T: AsyncRead,
{
    fn from(reader: T) -> Self {
        Self::new(reader)
    }
}

impl<T> futures::stream::Stream for ReadJsonStream<T>
where
    T: AsyncRead,
{
    type Item = alloy_json_rpc::PubSubItem;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        // Deserialize any buffered items.
        if !this.buf.is_empty() {
            this.reader.consume(this.buf.len());

            tracing::debug!(buf_len = this.buf.len(), "Deserializing buffered IPC data");
            let mut de = serde_json::Deserializer::from_slice(this.buf.as_ref()).into_iter();

            let item = de.next();
            match item {
                Some(Ok(response)) => {
                    this.items.push(response);
                }
                Some(Err(e)) => {
                    tracing::error!(%e, "IPC response contained invalid JSON. Buffer contents will be logged at trace level");
                    tracing::trace!(
                        buffer = %String::from_utf8_lossy(this.buf.as_ref()),
                        "IPC response contained invalid JSON. NOTE: Buffer contents do not include invalid utf8.",
                    );

                    return Ready(None);
                }
                None => {}
            }
            this.buf.advance(de.byte_offset());
            cx.waker().wake_by_ref();
            return Pending;
        }

        // Return any buffered items, rewaking.
        if !this.items.is_empty() {
            // may have more work!
            cx.waker().wake_by_ref();
            return Ready(this.items.pop());
        }

        tracing::debug!(buf_len = this.buf.len(), "Polling IPC socket for data");

        let data = ready!(this.reader.poll_fill_buf(cx));
        match data {
            Err(e) => {
                tracing::error!(%e, "Failed to read from IPC socket, shutting down");
                Ready(None)
            }
            Ok(data) => {
                tracing::debug!(data_len = data.len(), "Read data from IPC socket");
                this.buf.extend_from_slice(data);
                // wake task to run deserialization
                cx.waker().wake_by_ref();
                Pending
            }
        }
    }
}
