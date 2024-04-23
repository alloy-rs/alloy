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

#[macro_use]
extern crate tracing;

use bytes::{Buf, BytesMut};
use futures::{ready, AsyncRead, AsyncWriteExt, StreamExt};
use interprocess::local_socket::{tokio::LocalSocketStream, ToLocalSocketName};
use std::task::Poll::Ready;
use tokio::select;
use tokio_util::compat::FuturesAsyncReadCompatExt;

mod connect;
pub use connect::IpcConnect;

#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "mock")]
pub use mock::MockIpcServer;

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
                                if let Err(err) = writer.write_all(bytes.as_bytes()).await {
                                    error!(%err, "Failed to write to IPC socket");
                                    break true;
                                }
                            },
                            // dispatcher has gone away, or shutdown was received
                            None => {
                                debug!("Frontend has gone away");
                                break false;
                            },
                        }
                    }
                    // Read from the socket.
                    item = read.next() => {
                        match item {
                            Some(item) => {
                                if self.interface.send_to_frontend(item).is_err() {
                                    debug!("Frontend has gone away");
                                    break false;
                                }
                            }
                            None => {
                                error!("Read stream has failed.");
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

/// Default capacity for the IPC buffer.
const CAPACITY: usize = 4096;

/// A stream of JSON-RPC items, read from an [`AsyncRead`] stream.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct ReadJsonStream<T> {
    /// The underlying reader.
    #[pin]
    reader: tokio_util::compat::Compat<T>,
    /// A buffer for reading data from the reader.
    buf: BytesMut,
    /// Whether the buffer has been drained.
    drained: bool,
}

impl<T> ReadJsonStream<T>
where
    T: AsyncRead,
{
    fn new(reader: T) -> Self {
        Self { reader: reader.compat(), buf: BytesMut::with_capacity(CAPACITY), drained: true }
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
        use tokio_util::io::poll_read_buf;

        let mut this = self.project();

        loop {
            // try decoding from the buffer, but only if we have new data
            if !*this.drained {
                debug!(buf_len = this.buf.len(), "Deserializing buffered IPC data");
                let mut de = serde_json::Deserializer::from_slice(this.buf.as_ref()).into_iter();

                let item = de.next();

                // advance the buffer
                this.buf.advance(de.byte_offset());

                match item {
                    Some(Ok(response)) => {
                        return Ready(Some(response));
                    }
                    Some(Err(err)) => {
                        if err.is_data() {
                            trace!(
                                buffer = %String::from_utf8_lossy(this.buf.as_ref()),
                                "IPC buffer contains invalid JSON data",
                            );

                            if this.buf.len() > CAPACITY {
                                // buffer is full, we can't decode any more
                                error!("IPC response too large to decode");
                                return Ready(None);
                            }

                            // this happens if the deserializer is unable to decode a partial object
                            *this.drained = true;
                        } else if err.is_eof() {
                            trace!("partial object in IPC buffer");
                            // nothing decoded
                            *this.drained = true;
                        } else {
                            error!(%err, "IPC response contained invalid JSON. Buffer contents will be logged at trace level");
                            trace!(
                                buffer = %String::from_utf8_lossy(this.buf.as_ref()),
                                "IPC response contained invalid JSON. NOTE: Buffer contents do not include invalid utf8.",
                            );

                            return Ready(None);
                        }
                    }
                    None => {
                        // nothing decoded
                        *this.drained = true;
                    }
                }
            }

            // read more data into the buffer
            match ready!(poll_read_buf(this.reader.as_mut(), cx, &mut this.buf)) {
                Ok(0) => {
                    // stream is no longer readable and we're also unable to decode any more
                    // data. This happens if the IPC socket is closed by the other end.
                    // so we can return `None` here.
                    debug!("IPC socket EOF, stream is closed");
                    return Ready(None);
                }
                Ok(data_len) => {
                    debug!(%data_len, "Read data from IPC socket");
                    // can try decoding again
                    *this.drained = false;
                }
                Err(err) => {
                    error!(%err, "Failed to read from IPC socket, shutting down");
                    return Ready(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    #[tokio::test]
    async fn test_partial_stream() {
        let mock = tokio_test::io::Builder::new()
            // partial object
            .read(b"{\"jsonrpc\":\"2.0\",\"method\":\"eth_subscription\"")
            // trigger pending read
            .wait(std::time::Duration::from_millis(1))
            // complete object
            .read(r#", "params": {"subscription": "0xcd0c3e8af590364c09d0fa6a1210faf5", "result": {"difficulty": "0xd9263f42a87", "uncles": []}} }"#.as_bytes())
            .build();

        let mut reader = ReadJsonStream::new(mock.compat());
        poll_fn(|cx| {
            let res = reader.poll_next_unpin(cx);
            assert!(res.is_pending());
            Ready(())
        })
        .await;
        let _obj = reader.next().await.unwrap();
    }

    #[tokio::test]
    async fn test_large_invalid() {
        let mock = tokio_test::io::Builder::new()
            // partial object
            .read(b"{\"jsonrpc\":\"2.0\",\"method\":\"eth_subscription\"")
            // trigger pending read
            .wait(std::time::Duration::from_millis(1))
            // fill buffer with invalid data
            .read(vec![b'a'; CAPACITY].as_ref())
            .build();

        let mut reader = ReadJsonStream::new(mock.compat());
        poll_fn(|cx| {
            let res = reader.poll_next_unpin(cx);
            assert!(res.is_pending());
            Ready(())
        })
        .await;
        let obj = reader.next().await;
        assert!(obj.is_none());
    }
}
