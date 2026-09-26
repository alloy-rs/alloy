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
            // Parse each message as one-or-many items so a JSON-RPC batch
            // response (a single JSON array) is expanded into its individual
            // responses before being forwarded to the frontend.
            let mut read = ReadJsonStream::<_, alloy_json_rpc::PubSubItems>::new(read).fuse();

            let err = 'outer: loop {
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
                            Some(items) => {
                                for item in items {
                                    if self.interface.send_to_frontend(item).is_err() {
                                        debug!("Frontend has gone away");
                                        break 'outer false;
                                    }
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
    /// `true` while the current value is incomplete. Small frames that
    /// arrive in one read take the `serde_json` fast path; a partial parse
    /// flips this so subsequent polls skip the wasted probe and only scan.
    partial: bool,

    /// PhantomData marking the item type this stream will yield.
    _pd: std::marker::PhantomData<Item>,
}

impl<T: AsyncRead, U> ReadJsonStream<T, U> {
    fn new(reader: T) -> Self {
        Self {
            reader,
            buf: BytesMut::with_capacity(CAPACITY),
            scanner: FrameScanner::default(),
            partial: false,
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
            // Complete small frames: one serde pass, same cost as `main`.
            // A partial value flips `partial` so we do not re-parse the
            // growing buffer on every subsequent read.
            if !*this.partial && !this.buf.is_empty() {
                let mut de = serde_json::Deserializer::from_slice(this.buf.as_ref()).into_iter();
                match de.next() {
                    Some(Ok(item)) => {
                        this.buf.advance(de.byte_offset());
                        // The framer indexes into `buf`, so any advance resets it.
                        this.scanner.reset();
                        return Ready(Some(item));
                    }
                    // Incomplete, or complete but not an `Item`. Either way hand
                    // the buffer to the framer instead of re-parsing it.
                    Some(Err(err)) if err.is_eof() || err.is_data() => *this.partial = true,
                    Some(Err(err)) => {
                        log_invalid_json(&err, this.buf.as_ref());
                        return Ready(None);
                    }
                    None => {}
                }
            }

            match this.scanner.scan(this.buf.as_ref()) {
                Scan::Frame(end) => {
                    debug!(buf_len = this.buf.len(), end, "Framed IPC JSON value");
                    let frame = &this.buf[..end];
                    match serde_json::from_slice::<Item>(frame) {
                        Ok(item) => {
                            this.buf.advance(end);
                            this.scanner.reset();
                            *this.partial = false;
                            return Ready(Some(item));
                        }
                        Err(err) => {
                            log_invalid_json(&err, frame);
                            return Ready(None);
                        }
                    }
                }
                Scan::Incomplete => {
                    // Reserve before reading so `poll_read_buf` always gets a
                    // full page to fill, not whatever slack is left over.
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
                Scan::InvalidStart => {
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

/// Outcome of a [`FrameScanner::scan`] pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scan {
    /// A complete top-level value ends at this exclusive offset.
    Frame(usize),
    /// The buffer ends before the value does.
    Incomplete,
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
    /// Total bytes inspected across all [`Self::scan`] calls. Locks the O(n)
    /// guarantee in tests; not compiled into release builds.
    #[cfg(test)]
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
    /// Bytes already inspected are not re-examined on the next call, so a value
    /// split across reads still costs one pass over its bytes in total.
    fn scan(&mut self, buf: &[u8]) -> Scan {
        #[cfg(test)]
        let start = self.pos;
        let scan = self.scan_from(buf);
        #[cfg(test)]
        {
            self.examined += self.pos - start;
        }
        scan
    }

    fn scan_from(&mut self, buf: &[u8]) -> Scan {
        while self.pos < buf.len() {
            if self.in_string {
                if !self.skip_string(buf) {
                    return Scan::Incomplete;
                }
                continue;
            }

            if self.depth == 0 {
                match buf[self.pos] {
                    b' ' | b'\t' | b'\n' | b'\r' => {
                        self.pos += 1;
                    }
                    b'{' | b'[' => {
                        self.depth += 1;
                        self.pos += 1;
                    }
                    _ => {
                        self.pos += 1;
                        return Scan::InvalidStart;
                    }
                }
                continue;
            }

            // Inside a value: SIMD-skip to the next structural byte.
            let Some(rel) = next_structural(&buf[self.pos..]) else {
                self.pos = buf.len();
                return Scan::Incomplete;
            };
            self.pos += rel;
            match buf[self.pos] {
                b'"' => {
                    self.in_string = true;
                    self.pos += 1;
                }
                b'{' | b'[' => {
                    self.depth += 1;
                    self.pos += 1;
                }
                b'}' | b']' => {
                    self.depth -= 1;
                    self.pos += 1;
                    if self.depth == 0 {
                        return Scan::Frame(self.pos);
                    }
                }
                _ => unreachable!("next_structural only yields {{}}[]\""),
            }
        }
        Scan::Incomplete
    }

    /// Advance past the current JSON string. Returns `false` when `buf` ends
    /// before the closing quote (including a trailing unconsumed `\` — that
    /// sets [`Self::escaped`] so the next call resumes correctly).
    fn skip_string(&mut self, buf: &[u8]) -> bool {
        if self.escaped {
            if self.pos >= buf.len() {
                return false;
            }
            self.escaped = false;
            self.pos += 1;
        }

        while self.pos < buf.len() {
            let Some(rel) = memchr::memchr2(b'"', b'\\', &buf[self.pos..]) else {
                self.pos = buf.len();
                return false;
            };
            let idx = self.pos + rel;
            if buf[idx] != b'\\' {
                self.pos = idx + 1;
                self.in_string = false;
                return true;
            }
            // Skip the escaped byte too, deferring if it has not arrived yet.
            if idx + 1 >= buf.len() {
                self.pos = buf.len();
                self.escaped = true;
                return false;
            }
            self.pos = idx + 2;
        }
        false
    }
}

/// Offset of the next `{`, `}`, `[`, `]`, or `"` in `hay`, or `None`.
///
/// `memchr` takes at most three needles, so this scans for the openers and the
/// quote first, then for a closer within whatever prefix that left.
fn next_structural(hay: &[u8]) -> Option<usize> {
    match memchr::memchr3(b'"', b'{', b'[', hay) {
        Some(open) => memchr::memchr2(b'}', b']', &hay[..open]).or(Some(open)),
        None => memchr::memchr2(b'}', b']', hay),
    }
}

/// Log a payload that failed to deserialize, before ending the stream.
fn log_invalid_json(err: &serde_json::Error, buf: &[u8]) {
    error!(%err, "IPC response contained invalid JSON. Buffer contents will be logged at trace level");
    trace!(
        buffer = %String::from_utf8_lossy(buf),
        "IPC response contained invalid JSON. NOTE: Buffer contents do not include invalid utf8.",
    );
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

    fn serde_frame_end(buf: &[u8]) -> usize {
        let mut de = serde_json::Deserializer::from_slice(buf).into_iter::<Box<RawValue>>();
        de.next().expect("no value").expect("not well-formed");
        de.byte_offset()
    }

    #[test]
    fn frame_scanner_table() {
        struct Case {
            input: &'static [u8],
            expected: Scan,
        }

        let cases = [
            Case { input: b"", expected: Scan::Incomplete },
            Case { input: b"   \n\t", expected: Scan::Incomplete },
            Case { input: br#"{"a":1}"#, expected: Scan::Frame(7) },
            Case { input: br#"[1,2]"#, expected: Scan::Frame(5) },
            Case { input: br#"  {"a":1}"#, expected: Scan::Frame(9) },
            Case { input: br#"{"a":{"b":[1,2]}}"#, expected: Scan::Frame(17) },
            Case { input: br#"{"a":1}{"b":2}"#, expected: Scan::Frame(7) },
            Case { input: br#"{"a":"}][{"}"#, expected: Scan::Frame(12) },
            Case { input: br#"{"a":"\""}"#, expected: Scan::Frame(10) },
            Case { input: br#"{"a":"\\"}"#, expected: Scan::Frame(10) },
            Case { input: b"too many requests", expected: Scan::InvalidStart },
            Case { input: b"1", expected: Scan::InvalidStart },
            Case { input: br#""hi""#, expected: Scan::InvalidStart },
            Case { input: br#"{"a":1"#, expected: Scan::Incomplete },
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
            let got = scanner.scan(&frame[..=i]);
            if i + 1 < frame.len() {
                assert_eq!(got, Scan::Incomplete, "must not emit before the last byte (i={i})");
            } else {
                assert_eq!(got, Scan::Frame(frame.len()));
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
            let expected = Scan::Frame(serde_frame_end(raw));
            assert_eq!(scanner.scan(raw), expected, "input={}", String::from_utf8_lossy(raw));
        }
    }

    #[test]
    fn frame_scanner_is_linear_on_large_partial_buffer() {
        let mut frame = Vec::from(br#"{"jsonrpc":"2.0","id":1,"result":"0x"#);
        frame.extend(vec![b'0'; 4 * 1024 * 1024]);
        frame.extend_from_slice(br#""}"#);

        let mut scanner = FrameScanner::default();
        const CHUNK: usize = 4096;
        let mut end = Scan::Incomplete;
        for i in (CHUNK..=frame.len()).step_by(CHUNK) {
            end = scanner.scan(&frame[..i]);
        }
        if frame.len() % CHUNK != 0 {
            end = scanner.scan(&frame);
        }
        assert_eq!(end, Scan::Frame(frame.len()));
        assert!(
            scanner.examined <= frame.len().saturating_add(CHUNK),
            "examined {} bytes for a {}-byte frame; scan must be linear",
            scanner.examined,
            frame.len()
        );
        assert!(scanner.examined >= frame.len(), "scanner should inspect every byte of the frame");
    }

    #[test]
    fn frame_scanner_concatenated_small_frames_examine_once() {
        let frame = br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
        let n = 64usize;
        let mut buf = Vec::with_capacity(frame.len() * n);
        for _ in 0..n {
            buf.extend_from_slice(frame);
        }

        let mut scanner = FrameScanner::default();
        let mut offset = 0usize;
        let mut count = 0usize;
        while offset < buf.len() {
            scanner.reset();
            let Scan::Frame(end) = scanner.scan(&buf[offset..]) else {
                panic!("each concatenated frame is complete");
            };
            offset += end;
            count += 1;
        }

        assert_eq!(count, n);
        assert!(
            scanner.examined >= buf.len(),
            "scanner should inspect every byte ({})",
            scanner.examined
        );
        assert!(
            scanner.examined <= buf.len().saturating_add(n),
            "examined {} bytes for {} bytes of frames; must not rescan",
            scanner.examined,
            buf.len()
        );
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
