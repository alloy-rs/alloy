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

use futures::stream::{FuturesUnordered, StreamExt};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, oneshot},
};

use alloy_json_rpc::Response;

use crate::ReadJsonStream;

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
        let (read, mut writer) = stream.into_split();
        let mut reader = ReadJsonStream::new(read);

        // Channel for sending responses back to the writer
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(32);

        // Collection of in-flight request handlers
        let mut request_handlers = FuturesUnordered::new();

        debug!("Starting connection handler loop");
        loop {
            tokio::select! {
                // Handle incoming requests
                maybe_request = reader.next() => {
                    match maybe_request {
                        Some(request) => {
                            if let Ok(request) = serde_json::from_value::<Value>(request) {
                                debug!(
                                    id = ?request.get("id"),
                                    method = ?request.get("method"),
                                    "Received JSON-RPC request"
                                );

                                // Clone what we need for the task
                                let inner = inner.clone();
                                let tx = tx.clone();

                                // Spawn a new task to handle this request
                                request_handlers.push(tokio::spawn(async move {
                                    // Get the next queued response or generate an error
                                    let response = inner.replies.lock().pop_front().map_or_else(|| {
                                        warn!("No queued response available for request");
                                        serde_json::to_vec(&json!({
                                            "jsonrpc": "2.0",
                                            "id": request.get("id"),
                                            "error": {
                                                "code": -32603,
                                                "message": "No response queued"
                                            }
                                        })).expect("JSON serialization cannot fail")
                                    }, |response| {
                                        trace!(response_len = response.len(), "Using queued response");
                                        response
                                    });

                                    // Send response back to writer
                                    if tx.send(response).await.is_err() {
                                        warn!("Failed to send response - connection likely closed");
                                    }
                                }));
                            }
                        }
                        None => {
                            debug!("Reader stream ended");
                            break;
                        }
                    }
                }

                // Clean up completed request handlers
                Some(result) = request_handlers.next() => {
                    if let Err(e) = result {
                        error!(?e, "Request handler task failed");
                    }
                }

                // Write responses
                Some(response) = rx.recv() => {
                    writer.write_all(&response).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                    debug!("Response sent successfully");
                }
            }
        }

        // Wait for all in-flight request handlers to complete
        while let Some(result) = request_handlers.next().await {
            if let Err(e) = result {
                error!(?e, "Request handler task failed during shutdown");
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

        let inner = self.inner;

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

#[cfg(test)]
mod tests {
    use tokio::{net::UnixStream, task::JoinSet};

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

    /// Test concurrent request handling:
    /// 1. Multiple simultaneous client connections
    /// 2. Multiple requests per connection
    /// 3. Verify responses are received correctly
    /// 4. Verify response ordering within each connection
    #[tokio::test]
    async fn test_concurrent_requests() -> std::io::Result<()> {
        let server = MockIpcServer::new();
        let path = server.path();
        let handle = server.spawn().await?;

        // Queue multiple responses
        for i in 0..10 {
            handle
                .add_reply(json!({
                    "jsonrpc": "2.0",
                    "id": i,
                    "result": format!("0x{:x}", i * 100)
                }))
                .unwrap();
        }

        // Create multiple client connections
        let mut tasks = JoinSet::new();

        // Spawn 3 concurrent clients
        for client_id in 0..3 {
            let path = path.clone();
            tasks.spawn(async move {
                // Connect to server and split stream
                let stream = UnixStream::connect(path).await?;
                let (read, mut write) = stream.into_split();
                let mut reader = crate::ReadJsonStream::new(read);
                let mut responses = Vec::new();

                // Send multiple requests with slight delays to test interleaving
                for i in 0..3 {
                    let request_id = client_id * 3 + i;
                    let request = json!({
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "method": "eth_getBalance",
                        "params": [format!("0x{:x}", request_id)]
                    });

                    // Write request
                    write.write_all(&serde_json::to_vec(&request)?).await?;
                    write.write_all(b"\n").await?;
                    write.flush().await?;

                    // Read response
                    if let Some(response) = reader.next().await {
                        responses.push(response);
                    }
                }

                Ok::<Vec<Value>, std::io::Error>(responses)
            });
        }

        // Collect and verify all responses
        let mut all_responses = Vec::new();
        while let Some(result) = tasks.join_next().await {
            let responses = result.unwrap()?;
            all_responses.extend(responses);
        }

        // Verify we got all expected responses
        assert_eq!(all_responses.len(), 9); // 3 clients * 3 requests each

        // Verify responses are correct
        for response in all_responses {
            let id = response["id"].as_u64().unwrap();
            let expected = format!("0x{:x}", id * 100);
            assert_eq!(response["result"], expected);
        }

        handle.shutdown();
        Ok(())
    }
}
