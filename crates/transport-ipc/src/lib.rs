#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

use bytes::{Buf, BytesMut};
use futures::{ready, StreamExt};
use interprocess::local_socket::{tokio::prelude::*, Name};
use std::task::Poll::Ready;
use tokio::{
    io::{AsyncRead, AsyncWriteExt},
    select,
};
use tokio_util::io::poll_read_buf;

mod connect;
pub use connect::IpcConnect;

#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "mock")]
pub use mock::MockIpcServer;

type Result<T> = std::result::Result<T, std::io::Error>;

/// An IPC backend task.
struct IpcBackend {
    pub(crate) stream: LocalSocketStream,

    pub(crate) interface: alloy_pubsub::ConnectionInterface,
}

impl IpcBackend {
    /// Connect to a local socket. Either a unix socket or a windows named pipe.
    async fn connect(name: Name<'_>) -> Result<alloy_pubsub::ConnectionHandle> {
        let stream = LocalSocketStream::connect(name).await?;
        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = Self { stream, interface };
        backend.spawn();
        Ok(handle)
    }

    fn spawn(mut self) {
        let fut = async move {
            let (read, mut writer) = self.stream.split();
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
                                // Stream ended; upstream already logged the cause (EOF or JSON error).
                                debug!("IPC read stream ended");
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

/// Why [`FrameScanner::scan`] could not produce a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanError {
    /// The first non-whitespace byte is neither `{` nor `[`.
    InvalidStart,
}

/// Resumable search for the end of the next top-level JSON value.
///
/// Persisting `pos` across polls is what makes framing O(n) in the frame size:
/// bytes are examined exactly once no matter how the socket chunks them.
#[derive(Clone, Debug, Default)]
struct FrameScanner {
    /// Next byte to inspect. An index into the current buffer.
    pos: usize,
    /// Nesting depth of `{` / `[`. Zero means we have not started a value, or
    /// we just finished one.
    depth: usize,
    /// True while inside a JSON string.
    in_string: bool,
    /// True when the previous in-string byte was an unconsumed `\`.
    escaped: bool,
    /// Total bytes inspected across all [`Self::scan`] calls. Used to lock the
    /// O(n) complexity guarantee in tests.
    examined: usize,
}

impl FrameScanner {
    /// Reset framing state after a complete value is consumed from the buffer.
    ///
    /// `examined` is left intact so tests can sum work across many frames.
    const fn reset(&mut self) {
        self.pos = 0;
        self.depth = 0;
        self.in_string = false;
        self.escaped = false;
    }

    /// Find the exclusive end offset of the next top-level JSON object or array.
    ///
    /// Returns `Ok(None)` when `buf` ends before the value is complete. Bytes
    /// already inspected are not re-examined on the next call.
    fn scan(&mut self, buf: &[u8]) -> std::result::Result<Option<usize>, ScanError> {
        let start = self.pos;
        let result = self.scan_from(buf);
        self.examined = self.examined.saturating_add(self.pos.saturating_sub(start));
        result
    }

    fn scan_from(&mut self, buf: &[u8]) -> std::result::Result<Option<usize>, ScanError> {
        while self.pos < buf.len() {
            let b = buf[self.pos];

            if self.in_string {
                if self.escaped {
                    self.escaped = false;
                } else if b == b'\\' {
                    self.escaped = true;
                } else if b == b'"' {
                    self.in_string = false;
                }
                self.pos += 1;
                continue;
            }

            match b {
                b' ' | b'\t' | b'\n' | b'\r' if self.depth == 0 => {
                    self.pos += 1;
                }
                b'"' => {
                    if self.depth == 0 {
                        self.pos += 1;
                        return Err(ScanError::InvalidStart);
                    }
                    self.in_string = true;
                    self.pos += 1;
                }
                b'{' | b'[' => {
                    self.depth += 1;
                    self.pos += 1;
                }
                b'}' | b']' => {
                    if self.depth == 0 {
                        self.pos += 1;
                        return Err(ScanError::InvalidStart);
                    }
                    self.depth -= 1;
                    self.pos += 1;
                    if self.depth == 0 {
                        return Ok(Some(self.pos));
                    }
                }
                _ if self.depth == 0 => {
                    self.pos += 1;
                    return Err(ScanError::InvalidStart);
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        Ok(None)
    }
}

/// A stream of JSON-RPC items, read from an [`AsyncRead`] stream.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct ReadJsonStream<T, Item = alloy_json_rpc::PubSubItem> {
    /// The underlying reader.
    #[pin]
    reader: T,
    /// A buffer for reading data from the reader.
    buf: BytesMut,
    /// Incremental object/array framer. Survives across polls so a large
    /// response is scanned once, not re-parsed from byte zero on every read.
    scanner: FrameScanner,

    /// PhantomData marking the item type this stream will yield.
    _pd: std::marker::PhantomData<Item>,
}

impl<T: AsyncRead, U> ReadJsonStream<T, U> {
    fn new(reader: T) -> Self {
        Self {
            reader,
            buf: BytesMut::with_capacity(CAPACITY),
            scanner: FrameScanner::default(),
            _pd: core::marker::PhantomData,
        }
    }

    /// Total bytes the framer has inspected. Test-only complexity probe.
    #[cfg(test)]
    const fn bytes_examined(&self) -> usize {
        self.scanner.examined
    }
}

impl<T: AsyncRead, U> From<T> for ReadJsonStream<T, U> {
    fn from(reader: T) -> Self {
        Self::new(reader)
    }
}

impl<T: AsyncRead, Item> futures::stream::Stream for ReadJsonStream<T, Item>
where
    Item: serde::de::DeserializeOwned,
{
    type Item = Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.scanner.scan(this.buf.as_ref()) {
                Ok(Some(end)) => {
                    debug!(buf_len = this.buf.len(), end, "Framed IPC JSON value");
                    let frame = &this.buf[..end];
                    match serde_json::from_slice::<Item>(frame) {
                        Ok(item) => {
                            this.buf.advance(end);
                            this.scanner.reset();
                            return Ready(Some(item));
                        }
                        Err(err) => {
                            error!(
                                %err,
                                "IPC response contained invalid JSON. Buffer contents will be logged at trace level"
                            );
                            trace!(
                                buffer = %String::from_utf8_lossy(frame),
                                "IPC response contained invalid JSON. NOTE: Buffer contents do not include invalid utf8.",
                            );
                            this.buf.advance(end);
                            this.scanner.reset();
                            return Ready(None);
                        }
                    }
                }
                Ok(None) => {
                    // Need more bytes. Reserve so `poll_read_buf` can fill a
                    // full page; `BytesMut::chunk_mut` otherwise yields ~64 B.
                    this.buf.reserve(CAPACITY);
                    match ready!(poll_read_buf(this.reader.as_mut(), cx, this.buf)) {
                        Ok(0) => {
                            debug!("IPC socket EOF, stream is closed");
                            return Ready(None);
                        }
                        Ok(data_len) => {
                            debug!(%data_len, "Read data from IPC socket");
                        }
                        Err(err) => {
                            error!(%err, "Failed to read from IPC socket, shutting down");
                            return Ready(None);
                        }
                    }
                }
                Err(ScanError::InvalidStart) => {
                    error!("IPC stream contained a non-object, non-array JSON value");
                    trace!(
                        buffer = %String::from_utf8_lossy(this.buf.as_ref()),
                        "IPC stream contained a non-object, non-array JSON value. NOTE: Buffer contents do not include invalid utf8.",
                    );
                    return Ready(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::{PubSubItem, ResponsePayload};
    use serde_json::value::RawValue;
    use std::future::poll_fn;

    const JSON_RPC_ERROR: &[u8] = br#"{
        "jsonrpc": "2.0",
        "id": 1766,
        "error": {
            "code": -32000,
            "message": "filter not found"
        }
    }"#;

    fn serde_frame_end(buf: &[u8]) -> Option<usize> {
        let mut de = serde_json::Deserializer::from_slice(buf).into_iter::<Box<RawValue>>();
        match de.next() {
            Some(Ok(_)) => Some(de.byte_offset()),
            _ => None,
        }
    }

    #[test]
    fn frame_scanner_table() {
        struct Case {
            input: &'static [u8],
            expected: std::result::Result<Option<usize>, ScanError>,
        }

        let cases = [
            Case { input: b"", expected: Ok(None) },
            Case { input: b"   \n\t", expected: Ok(None) },
            Case { input: br#"{"a":1}"#, expected: Ok(Some(7)) },
            Case { input: br#"[1,2]"#, expected: Ok(Some(5)) },
            Case { input: br#"  {"a":1}"#, expected: Ok(Some(9)) },
            Case { input: br#"{"a":{"b":[1,2]}}"#, expected: Ok(Some(17)) },
            Case { input: br#"{"a":1}{"b":2}"#, expected: Ok(Some(7)) },
            Case { input: br#"{"a":"}][{"}"#, expected: Ok(Some(12)) },
            Case { input: br#"{"a":"\""}"#, expected: Ok(Some(10)) },
            Case { input: br#"{"a":"\\"}"#, expected: Ok(Some(10)) },
            Case { input: b"too many requests", expected: Err(ScanError::InvalidStart) },
            Case { input: b"1", expected: Err(ScanError::InvalidStart) },
            Case { input: br#""hi""#, expected: Err(ScanError::InvalidStart) },
            Case { input: br#"{"a":1"#, expected: Ok(None) },
        ];

        for (i, case) in cases.iter().enumerate() {
            let mut scanner = FrameScanner::default();
            let got = scanner.scan(case.input);
            assert_eq!(got, case.expected, "case {i}: {}", String::from_utf8_lossy(case.input));
        }
    }

    #[test]
    fn frame_scanner_resumes_byte_by_byte() {
        let frame = br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
        let mut scanner = FrameScanner::default();
        for i in 0..frame.len() {
            let got = scanner.scan(&frame[..=i]).unwrap();
            if i + 1 < frame.len() {
                assert_eq!(got, None, "must not emit before the last byte (i={i})");
            } else {
                assert_eq!(got, Some(frame.len()));
            }
        }
        assert_eq!(scanner.examined, frame.len());
    }

    #[test]
    fn frame_scanner_matches_serde_byte_offset() {
        let corpus: &[&[u8]] = &[
            br#"{"jsonrpc":"2.0","id":1,"result":"0xaaa"}"#,
            JSON_RPC_ERROR,
            br#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0x1","result":{"difficulty":"0x1","uncles":[]}}}"#,
            br#"  {"a":1}"#,
            br#"{"a":1}{"b":2}"#,
            br#"{"s":"quote \" and brace } and bracket ]"}"#,
            br#"{"s":"trailing backslash \\"}"#,
            br#"[1,{"a":[2,3]}]"#,
            br#"{"nested":{"x":[1,{"y":"}"}]}}"#,
        ];

        for raw in corpus {
            let mut scanner = FrameScanner::default();
            let scanned = scanner.scan(raw).expect("corpus entries are well-formed objects/arrays");
            let serde_end = serde_frame_end(raw);
            assert_eq!(scanned, serde_end, "input={}", String::from_utf8_lossy(raw));
        }
    }

    #[test]
    fn frame_scanner_is_linear_on_large_partial_buffer() {
        let mut frame = Vec::from(br#"{"jsonrpc":"2.0","id":1,"result":"0x"#);
        frame.extend(vec![b'0'; 4 * 1024 * 1024]);
        frame.extend_from_slice(br#""}"#);

        let mut scanner = FrameScanner::default();
        const CHUNK: usize = 4096;
        let mut end = None;
        for i in (CHUNK..=frame.len()).step_by(CHUNK) {
            end = scanner.scan(&frame[..i]).unwrap();
        }
        if frame.len() % CHUNK != 0 {
            end = scanner.scan(&frame).unwrap();
        }
        assert_eq!(end, Some(frame.len()));
        assert!(
            scanner.examined <= frame.len().saturating_add(CHUNK),
            "examined {} bytes for a {}-byte frame; scan must be linear",
            scanner.examined,
            frame.len()
        );
        assert!(scanner.examined >= frame.len(), "scanner should inspect every byte of the frame");
    }

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

        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
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

        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        poll_fn(|cx| {
            let res = reader.poll_next_unpin(cx);
            assert!(res.is_pending());
            Ready(())
        })
        .await;
        let obj = reader.next().await;
        assert!(obj.is_none());
    }

    #[tokio::test]
    async fn test_large_valid() {
        let header = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x";
        let filling_zeros = header
            .iter()
            .chain(vec![b'0'; CAPACITY - header.len()].iter())
            .copied()
            .collect::<Vec<_>>();

        let first_page = filling_zeros.as_ref();
        let second_page = b"\"}";

        let mock = tokio_test::io::Builder::new()
            // partial object
            .read(first_page)
            // trigger pending read
            .wait(std::time::Duration::from_millis(1))
            // complete object
            .read(second_page)
            .build();

        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        poll_fn(|cx| {
            let res = reader.poll_next_unpin(cx);
            assert!(res.is_pending());
            Ready(())
        })
        .await;
        let obj = reader.next().await;
        assert!(obj.is_some());
    }

    #[tokio::test]
    async fn json_rpc_error_object_is_pubsub_response() {
        let mock = tokio_test::io::Builder::new().read(JSON_RPC_ERROR).build();
        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        let item = reader.next().await.expect("framed JSON-RPC error object");

        match item {
            PubSubItem::Response(resp) => {
                assert_eq!(resp.id, 1766.into());
                match resp.payload {
                    ResponsePayload::Failure(err) => {
                        assert_eq!(err.code, -32000);
                        assert_eq!(err.message, "filter not found");
                    }
                    ResponsePayload::Success(_) => panic!("expected error payload"),
                }
            }
            PubSubItem::Notification(_) => panic!("expected response"),
        }
    }

    #[tokio::test]
    async fn concatenated_frames_are_framed_and_parsed() {
        let first = r#"{"jsonrpc":"2.0","id":1,"result":"0xaaa"}"#;
        let second = r#"{"jsonrpc":"2.0","id":2,"result":"0xbbb"}"#;
        let mut body = Vec::new();
        body.extend_from_slice(first.as_bytes());
        body.extend_from_slice(second.as_bytes());

        let mock = tokio_test::io::Builder::new().read(&body).build();
        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        let item1 = reader.next().await.expect("first frame");
        let item2 = reader.next().await.expect("second frame");

        match (item1, item2) {
            (PubSubItem::Response(r1), PubSubItem::Response(r2)) => {
                assert_eq!(r1.id, 1.into());
                assert_eq!(r2.id, 2.into());
            }
            _ => panic!("expected two responses"),
        }
    }

    #[tokio::test]
    async fn non_json_body_ends_stream() {
        let mock = tokio_test::io::Builder::new().read(b"too many requests").build();
        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        assert!(reader.next().await.is_none());
    }

    #[tokio::test]
    async fn wrong_shape_frame_ends_stream_instead_of_wedging() {
        // A complete JSON array is a valid frame but not a `PubSubItem`.
        // Advancing past it must terminate the stream; the old `is_data`
        // path left the bytes in place and spun.
        let mock = tokio_test::io::Builder::new().read(br#"[1,2,3]"#).build();
        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        assert!(reader.next().await.is_none());
    }

    #[tokio::test]
    async fn truncated_frame_never_yields_an_item() {
        let mock =
            tokio_test::io::Builder::new().read(br#"{"jsonrpc":"2.0","id":1,"result":"#).build();
        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        assert!(reader.next().await.is_none());
    }

    #[tokio::test]
    async fn chunked_large_frame_is_linear() {
        let header = br#"{"jsonrpc":"2.0","id":1,"result":"0x"#;
        let mut frame = Vec::from(*header);
        frame.extend(vec![b'0'; 4 * 1024 * 1024]);
        frame.extend_from_slice(br#""}"#);

        let chunks: Vec<&[u8]> = frame.chunks(4096).collect();
        let mut builder = tokio_test::io::Builder::new();
        for chunk in &chunks {
            builder.read(chunk);
        }
        let mock = builder.build();

        let mut reader = ReadJsonStream::<_, PubSubItem>::new(mock);
        let item = reader.next().await.expect("large frame should parse");
        match item {
            PubSubItem::Response(resp) => assert_eq!(resp.id, 1.into()),
            PubSubItem::Notification(_) => panic!("expected response"),
        }
        let examined = reader.bytes_examined();
        assert!(
            examined <= frame.len().saturating_add(8192),
            "examined {examined} bytes for a {}-byte frame; ReadJsonStream must not re-scan",
            frame.len()
        );
    }
}
