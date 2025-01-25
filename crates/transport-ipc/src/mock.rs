//! An IPC mock server and client for testing Ethereum JSON-RPC providers.
//! This module provides a way to mock responses from an IPC-based Ethereum node
//! by creating a Unix domain socket that responds with pre-configured replies.

use alloy_json_rpc::Response;
use serde::Serialize;
use serde_json::Value;
use std::{collections::VecDeque, sync::Arc};
use tempfile::NamedTempFile;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
    sync::{oneshot, Mutex},
};

/// A handle to control a running mock IPC server.
///
/// The handle allows adding responses that will be returned to the provider
/// and triggering server shutdown. Multiple handles can exist for the same
/// server instance.
#[derive(Debug, Clone)]
pub struct MockIpcHandle {
    /// Queue of responses to return to the provider
    replies: Arc<Mutex<VecDeque<Vec<u8>>>>,
    /// Channel for triggering server shutdown
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// Keeps the temporary socket file alive while the handle exists
    _temp_file: Arc<NamedTempFile>,
}

/// ```rust,no_run,ignore
/// 
///  let server = MockIpcServer::new();
///  let path = server.path();
///  let handle = server.spawn().await?;
///
///  // Queue a mock response for eth_getBalance
///  handle
///     .add_reply(json!({
///         "jsonrpc": "2.0",
///         "id": 0,
///         "result": "0x0000000000000000000000000000000000000000000000000000000000000064"
///   }))
///     .await;
///
///  // Create a provider connected to our mock
///  let provider = ProviderBuilder::new().on_ipc(IpcConnect::new(path)).await?;
///
///  // Make a request and verify the response
///  let balance = provider
///     .get_balance("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".parse()?)
///     .await?;
///
///  assert_eq!(balance, U256::from(100));
///
///  // Clean shutdown
///  handle.shutdown().await;
///
///  Ok(())
/// ```
impl MockIpcHandle {
    /// Add a raw byte vector as a response to be returned to the provider.
    /// Responses are returned in FIFO order.
    pub async fn add_raw_reply(&self, reply: Vec<u8>) {
        debug!(reply_len = reply.len(), "Adding raw reply");
        self.replies.lock().await.push_back(reply);
    }

    /// Add a serializable value as a response to be returned to the provider.
    /// The value will be serialized to JSON before being queued.
    /// This is the primary method for adding JSON-RPC responses.
    pub async fn add_reply<S: Serialize>(&self, s: S) {
        let reply = match serde_json::to_vec(&s) {
            Ok(r) => r,
            Err(e) => {
                error!(?e, "Failed to serialize reply");
                return;
            }
        };
        debug!(reply_len = reply.len(), "Adding JSON reply");
        self.add_raw_reply(reply).await;
    }

    /// Add a json-rpc response to the server.
    pub async fn add_response<S: Serialize>(&mut self, response: Response<S>) {
        self.add_reply(response).await;
    }

    /// Trigger a graceful shutdown of the mock IPC server.
    /// This will cause the server to stop accepting new connections and
    /// close the Unix domain socket.
    pub async fn shutdown(&self) {
        debug!("Shutting down server");
        if let Some(tx) = self.shutdown.lock().await.take() {
            let _ = tx.send(());
        }
    }
}

/// A mock IPC server that can be used to test providers without connecting to a real node.
///
/// The server creates a Unix domain socket and responds to JSON-RPC requests with
/// pre-configured replies. This allows testing provider implementations without needing
/// a real Ethereum node.
#[derive(Debug)]
pub struct MockIpcServer {
    /// Queue of responses to return to the provider
    replies: Arc<Mutex<VecDeque<Vec<u8>>>>,
    /// Temporary file used for the Unix domain socket
    temp_file: Arc<NamedTempFile>,
    /// Channel for triggering server shutdown
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl Default for MockIpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockIpcServer {
    /// Create a new mock IPC server instance.
    /// This creates a temporary file to use as the Unix domain socket path.
    pub fn new() -> Self {
        let temp_file = Arc::new(NamedTempFile::new().expect("Failed to create temp file"));
        let path = temp_file.path();

        // Clean up any existing socket file at this path
        if path.exists() {
            let _ = fs::remove_file(path);
        }

        Self {
            replies: Arc::new(Mutex::new(VecDeque::new())),
            temp_file,
            shutdown: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the path to the Unix domain socket that this server will listen on.
    /// This path should be passed to the provider being tested.
    pub fn path(&self) -> PathBuf {
        self.temp_file.path().to_owned()
    }

    /// Create a new handle to control this server.
    /// The handle can be used to add responses and trigger shutdown.
    pub fn handle(&self) -> MockIpcHandle {
        MockIpcHandle {
            replies: self.replies.clone(),
            shutdown: self.shutdown.clone(),
            _temp_file: self.temp_file.clone(),
        }
    }

    /// Handle a single client connection.
    /// This function runs in a separate task for each connected client.
    async fn handle_connection(
        mut stream: tokio::net::UnixStream,
        replies: Arc<Mutex<VecDeque<Vec<u8>>>>,
    ) {
        debug!("New connection established");

        let mut buf = [0u8; 4096];

        loop {
            match stream.read(&mut buf).await {
                Ok(0) => {
                    debug!("Connection closed by client");
                    break;
                }
                Ok(n) => {
                    // Parse the request data as a string
                    let request_str = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    debug!(request = %request_str, "Received request");

                    // Try to parse as JSON-RPC request
                    if let Ok(request) = serde_json::from_str::<Value>(&request_str) {
                        debug!(
                            id = ?request.get("id"),
                            method = ?request.get("method"),
                            "Parsed JSON-RPC request"
                        );

                        // Get and send the next queued response
                        if let Some(response) = replies.lock().await.pop_front() {
                            if let Err(e) = stream.write_all(&response).await {
                                error!(?e, "Failed to write response");
                                break;
                            }
                            if let Err(e) = stream.write_all(b"\n").await {
                                error!(?e, "Failed to write newline");
                                break;
                            }
                            if let Err(e) = stream.flush().await {
                                error!(?e, "Failed to flush");
                                break;
                            }
                            debug!("Response sent successfully");
                        } else {
                            // No response was queued, send an error
                            warn!("No queued response available");
                            let error_response = serde_json::to_vec(&json!({
                                "jsonrpc": "2.0",
                                "id": request.get("id"),
                                "error": {
                                    "code": -32603,
                                    "message": "No response queued"
                                }
                            }))
                            .unwrap();

                            if let Err(e) = stream.write_all(&error_response).await {
                                error!(?e, "Failed to write error response");
                                break;
                            }
                            if let Err(e) = stream.write_all(b"\n").await {
                                error!(?e, "Failed to write newline");
                                break;
                            }
                            if let Err(e) = stream.flush().await {
                                error!(?e, "Failed to flush");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(?e, "Failed to read from connection");
                    break;
                }
            }
        }
        debug!("Connection handler finished");
    }

    /// Start the mock IPC server.
    /// Returns a handle that can be used to control the server.
    /// The server will run until shutdown is triggered via the handle.
    pub async fn spawn(self) -> eyre::Result<MockIpcHandle> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown.lock().await = Some(shutdown_tx);
        let handle = self.handle();

        let socket_path = self.temp_file.path().to_owned();
        let listener = UnixListener::bind(&socket_path)?;

        let task_handle = handle.clone();

        // Spawn the main server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        debug!("Shutdown signal received");
                        break;
                    }
                    Ok((stream, _)) = listener.accept() => {
                        debug!("New connection accepted");
                        let replies = task_handle.replies.clone();
                        tokio::spawn(Self::handle_connection(stream, replies));
                    }
                }
            }
            debug!("Server shutdown complete");
        });

        Ok(handle)
    }
}
