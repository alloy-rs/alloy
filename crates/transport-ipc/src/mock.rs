//! Mock IPC server.

use alloy_json_rpc::Response;
use futures::{AsyncReadExt, AsyncWriteExt};
use serde::Serialize;
use std::{collections::VecDeque, path::PathBuf};
use tempfile::NamedTempFile;

/// Mock IPC server.
#[derive(Debug)]
pub struct MockIpcServer {
    /// Replies to send, in order
    replies: VecDeque<Vec<u8>>,
    /// Path to the socket
    path: NamedTempFile,
}

impl MockIpcServer {
    /// Create a new mock IPC server.
    pub fn new() -> Self {
        Self { replies: VecDeque::new(), path: NamedTempFile::new().unwrap() }
    }

    /// Add a raw reply to the server.
    pub fn add_raw_reply(&mut self, reply: Vec<u8>) {
        self.replies.push_back(reply);
    }

    /// Add a reply to the server.
    pub fn add_reply<S: Serialize>(&mut self, s: S) {
        let reply = serde_json::to_vec(&s).unwrap();
        self.add_raw_reply(reply);
    }

    /// Add a json-rpc response to the server.
    pub fn add_response<S: Serialize>(&mut self, response: Response<S>) {
        self.add_reply(response);
    }

    /// Get the path to the socket.
    pub fn path(&self) -> PathBuf {
        self.path.path().to_owned()
    }

    /// Run the server.
    pub async fn run(mut self) {
        let socket =
            interprocess::local_socket::tokio::LocalSocketStream::connect(self.path.path())
                .await
                .unwrap();

        let (mut reader, mut writer) = socket.into_split();

        let mut buf = [0u8; 4096];
        loop {
            reader.read(&mut buf).await.unwrap();
            let reply = self.replies.pop_front().unwrap();
            writer.write_all(&reply).await.unwrap();
        }
    }
}
