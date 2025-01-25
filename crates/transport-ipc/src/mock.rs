//! # Mock IPC server for testing Ethereum JSON-RPC providers
//!
//! This module provides functionality to create a mock IPC server that emulates an
//! Ethereum node's IPC interface. It's primarily intended for testing JSON-RPC provider
//! implementations without needing a real Ethereum node.
//!
//! ## Key Features
//!
//! - Create mock IPC servers that respond to JSON-RPC requests
//! - Pre-configure responses to be returned to clients
//! - Support for both raw and JSON-serialized responses
//! - Clean handling of socket file lifecycle
//! - Graceful shutdown support
//!
//! ## Example
//!
//! ```rust,no_run
//! use alloy_json_rpc::Response;
//! use serde_json::json;
//!
//! use alloy_transport_ipc::MockIpcServer;
//!
//! async fn example() -> std::io::Result<()> {
//! // Create and spawn server
//! let server = MockIpcServer::new();
//! let path = server.path();
//! let handle = server.spawn().await?;
//!
//! // Queue multiple responses
//! handle.add_reply(json!({
//!     "jsonrpc": "2.0",
//!     "id": 1,
//!     "result": "0x123"
//! }))?;
//!
//! handle.add_reply(json!({
//!     "jsonrpc": "2.0",
//!     "id": 2,
//!     "result": ["0x456", "0x789"]
//! }))?;
//!
//! // Add a raw response
//! handle.add_raw_reply(b"{\"jsonrpc\":\"2.0\",\"id\":3,\"result\":true}".to_vec());
//!
//! // Do the things
//!
//! // Shutdown when done
//! handle.shutdown();
//! # Ok(())
//! # }
//! ```

use std::{collections::VecDeque, fs, path::PathBuf, sync::Arc};

use futures::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use tokio::{io::AsyncWriteExt, sync::oneshot};

use alloy_json_rpc::Response;

/// Represents the shared state between the IPC server and its handles.
/// This state includes:
/// - A queue of pre-configured responses
/// - A shutdown signal channel
/// - A temporary file used for the Unix domain socket
#[derive(Debug)]
struct Inner {
    /// Queue of responses to be sent to clients
    replies: Mutex<VecDeque<Vec<u8>>>,
    /// Channel for triggering server shutdown
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    /// Temporary file backing the Unix domain socket
    temp_file: NamedTempFile,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Ensure socket file cleanup on drop
        // This is important for preventing resource leaks and socket file conflicts
        if let Ok(path) = self.temp_file.path().canonicalize() {
            if path.exists() {
                debug!(?path, "Cleaning up socket file on drop");
                let _ = fs::remove_file(&path);
            }
        }
    }
}

/// A handle to control a running mock IPC server.
///
/// This handle can be used to:
/// - Add responses that will be returned to clients
/// - Trigger server shutdown
/// - Multiple handles can exist for the same server instance
#[derive(Debug, Clone)]
pub struct MockIpcHandle {
    /// Reference to shared server state
    inner: Arc<Inner>,
}

impl MockIpcHandle {
    /// Add a raw byte vector as a response to be returned to clients.
    /// Responses are returned in FIFO order.
    ///
    /// # Arguments
    /// * `reply` - Raw bytes to send as response
    pub fn add_raw_reply(&self, reply: Vec<u8>) {
        debug!(reply_len = reply.len(), "Adding raw reply to response queue");
        self.inner.replies.lock().push_back(reply);
    }

    /// Add a serializable value as a response to be returned to clients.
    /// The value will be serialized to JSON before being queued.
    ///
    /// # Arguments
    /// * `s` - Any value that implements serde::Serialize
    ///
    /// # Returns
    /// * `Result<(), serde_json::Error>` - Ok if serialization succeeds
    pub fn add_reply<S: Serialize>(&self, s: S) -> Result<(), serde_json::Error> {
        let reply = serde_json::to_vec(&s)?;
        debug!(reply_len = reply.len(), "Adding JSON reply to response queue");
        self.add_raw_reply(reply);
        Ok(())
    }

    /// Add a JSON-RPC response to the server's response queue.
    ///
    /// # Arguments
    /// * `response` - A JSON-RPC Response object
    ///
    /// # Returns
    /// * `Result<(), serde_json::Error>` - Ok if serialization succeeds
    pub fn add_response<S: Serialize>(
        &mut self,
        response: Response<S>,
    ) -> Result<(), serde_json::Error> {
        self.add_reply(response)
    }

    /// Trigger a graceful shutdown of the mock IPC server.
    /// This will cause the server to stop accepting new connections and
    /// clean up its resources.
    pub fn shutdown(&self) {
        debug!("Initiating server shutdown");
        if let Some(tx) = self.inner.shutdown.lock().take() {
            let _ = tx.send(());
        }
    }
}

/// A mock IPC server that emulates an Ethereum JSON-RPC over IPC provider.
///
/// The server creates a Unix domain socket and responds to JSON-RPC requests with
/// pre-configured replies. This allows testing provider implementations without
/// needing a real Ethereum node.
#[derive(Debug)]
pub struct MockIpcServer {
    /// Reference to shared server state
    inner: Arc<Inner>,
}

impl Default for MockIpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockIpcServer {
    /// Create a new mock IPC server instance.
    /// This creates a temporary file to use as the Unix domain socket path.
    ///
    /// # Returns
    /// * A new `MockIpcServer` instance
    pub fn new() -> Self {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();
        debug!(?path, "Created new mock IPC server");

        let inner = Arc::new(Inner {
            replies: Mutex::new(VecDeque::new()),
            shutdown: Mutex::new(None),
            temp_file,
        });

        Self { inner }
    }

    /// Get the path to the Unix domain socket that this server will listen on.
    /// This path should be passed to the provider being tested.
    ///
    /// # Returns
    /// * `PathBuf` containing the socket path
    pub fn path(&self) -> PathBuf {
        self.inner.temp_file.path().to_owned()
    }

    /// Create a new handle to control this server.
    /// The handle can be used to add responses and trigger shutdown.
    ///
    /// # Returns
    /// * A new `MockIpcHandle` instance
    pub fn handle(&self) -> MockIpcHandle {
        MockIpcHandle { inner: self.inner.clone() }
    }

    /// Handle a single client connection.
    /// This function runs in a separate task for each connected client.
    ///
    /// # Arguments
    /// * `stream` - The Unix domain socket stream for the client
    /// * `inner` - Reference to shared server state
    async fn handle_connection(
        stream: tokio::net::UnixStream,
        inner: Arc<Inner>,
    ) -> std::io::Result<()> {
        use crate::ReadJsonStream;

        let (read, mut writer) = stream.into_split();
        let mut reader = ReadJsonStream::new(read);

        debug!("Starting connection handler loop");
        while let Some(request) = reader.next().await {
            if let Ok(request) = serde_json::from_value::<Value>(request) {
                debug!(
                    id = ?request.get("id"),
                    method = ?request.get("method"),
                    "Received JSON-RPC request"
                );

                // Get the next queued response or generate an error
                let response = if let Some(response) = inner.replies.lock().pop_front() {
                    trace!(response_len = response.len(), "Using queued response");
                    response
                } else {
                    warn!("No queued response available for request");
                    serde_json::to_vec(&json!({
                        "jsonrpc": "2.0",
                        "id": request.get("id"),
                        "error": {
                            "code": -32603,
                            "message": "No response queued"
                        }
                    }))?
                };

                // Send the response
                writer.write_all(&response).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                debug!("Response sent successfully");
            }
        }

        debug!("Connection handler completed");
        Ok(())
    }

    /// Start the mock IPC server.
    /// Returns a handle that can be used to control the server.
    /// The server will run until shutdown is triggered via the handle.
    ///
    /// # Returns
    /// * `Result<MockIpcHandle, std::io::Error>` - Handle to control the server
    pub async fn spawn(self) -> std::io::Result<MockIpcHandle> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.inner.shutdown.lock() = Some(shutdown_tx);
        let handle = self.handle();

        let socket_path = self.inner.temp_file.path().to_owned();

        // Clean up any existing socket file
        if socket_path.exists() {
            debug!(?socket_path, "Removing existing socket file");
            fs::remove_file(&socket_path)?;
        }

        // Create and bind the Unix domain socket
        let listener = tokio::net::UnixListener::bind(&socket_path).map_err(|e| {
            error!(?e, ?socket_path, "Failed to bind Unix socket");
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to bind Unix socket at {:?}: {}", socket_path, e),
            )
        })?;

        let inner = self.inner.clone();

        // Spawn the main server task
        tokio::spawn(async move {
            debug!("Starting main server loop");
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        debug!("Shutdown signal received");
                        break;
                    }
                    Ok((stream, _)) = listener.accept() => {
                        debug!("New client connection accepted");
                        let inner = inner.clone();
                        tokio::spawn(Self::handle_connection(stream, inner));
                    }
                }
            }
            debug!("Server shutdown complete");

            // Clean up the socket file
            if let Ok(path) = socket_path.canonicalize() {
                if path.exists() {
                    debug!(?path, "Cleaning up socket file on shutdown");
                    let _ = fs::remove_file(&path);
                }
            }
        });

        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::UnixStream;

    use super::*;

    /// Test basic server functionality:
    /// 1. Server creation and startup
    /// 2. Adding a response
    /// 3. Client connection and request
    /// 4. Response verification
    /// 5. Server shutdown
    #[tokio::test]
    async fn test_mock_ipc_server() -> std::io::Result<()> {
        let server = MockIpcServer::new();
        let path = server.path();
        let handle = server.spawn().await?;

        // Queue a test response
        handle
            .add_reply(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x123"
            }))
            .unwrap();

        // Connect and send request
        let mut stream = UnixStream::connect(path).await?;
        stream.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_getBalance\"}\n").await?;

        // Read and verify response
        let mut reader = crate::ReadJsonStream::new(stream);
        let response: Value = reader.next().await.unwrap();
        assert_eq!(response["result"], "0x123");

        handle.shutdown();
        Ok(())
    }
}
