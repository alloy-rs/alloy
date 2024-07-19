//! Generic node bindings to a node.

use alloy_genesis::Genesis;
use std::{
    path::PathBuf,
    process::{Child, ChildStderr},
    time::Duration,
};
use thiserror::Error;
use url::Url;

/// How long we will wait for geth to indicate that it is ready.
pub const NODE_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for waiting for geth to add a peer.
pub const NODE_DIAL_LOOP_TIMEOUT: Duration = Duration::from_secs(20);

/// Errors that can occur when working with the [`GethInstance`].
#[derive(Debug)]
pub enum NodeInstanceError {
    /// Timed out waiting for a message from geth's stderr.
    Timeout(String),

    /// A line could not be read from the geth stderr.
    ReadLineError(std::io::Error),

    /// The child geth process's stderr was not captured.
    NoStderr,
}

/// Configuration for a node.
#[derive(Debug)]
pub struct NodeConfig {
    /// The port of the node.
    pub port: u16,
    /// The p2p port of the node.
    pub p2p_port: Option<u16>,
    /// The data directory of the node.
    pub data_dir: Option<PathBuf>,
    /// The genesis configuration of the node.
    pub genesis: Option<Genesis>,
    /// The IPC path of the node.
    pub ipc: Option<PathBuf>,
}

/// A node instance. Will close the instance when dropped.
///
/// Construct this using [`Node`].
pub trait NodeInstance {
    /// Returns the configuration of this instance.
    fn config(&self) -> &NodeConfig;

    /// Returns the child process of this instance.
    fn pid(&mut self) -> &mut Child;

    /// Returns the port of this instance.
    fn port(&self) -> u16 {
        self.config().port
    }

    /// Returns the p2p port of this instance.
    fn p2p_port(&self) -> Option<u16> {
        self.config().p2p_port
    }

    /// Returns the path to this instances' data directory.
    fn data_dir(&self) -> &Option<PathBuf> {
        &self.config().data_dir
    }

    /// Returns the genesis configuration used to configure this instance.
    fn genesis(&self) -> &Option<Genesis> {
        &self.config().genesis
    }

    /// Returns the IPC path of this instance.
    fn ipc(&self) -> &Option<PathBuf> {
        &self.config().ipc
    }

    /// Returns the HTTP endpoint of this instance.
    #[doc(alias = "http_endpoint")]
    fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.config().port)
    }

    /// Returns the Websocket endpoint of this instance.
    fn ws_endpoint(&self) -> String {
        format!("ws://localhost:{}", self.config().port)
    }

    /// Returns the IPC endpoint of this instance.
    fn ipc_endpoint(&self) -> String;

    /// Returns the HTTP endpoint url of this instance.
    #[doc(alias = "http_endpoint_url")]
    fn endpoint_url(&self) -> Url {
        Url::parse(&self.endpoint()).unwrap()
    }

    /// Returns the Websocket endpoint url of this instance.
    fn ws_endpoint_url(&self) -> Url {
        Url::parse(&self.ws_endpoint()).unwrap()
    }

    /// Blocks until the node has added specified peer.
    fn wait_to_add_peer(&mut self, id: &str) -> Result<(), NodeInstanceError>;

    /// Takes the stderr contained in the child process.
    ///
    /// This leaves a `None` in its place, so calling methods that require a stderr to be present
    /// will fail if called after this.
    fn stderr(&mut self) -> Result<ChildStderr, NodeInstanceError> {
        self.pid().stderr.take().ok_or(NodeInstanceError::NoStderr)
    }
}

/// Errors that can occur when working with the [`Node`].
#[derive(Debug, Error)]
pub enum NodeError {
    /// The chain id was not set.
    #[error("the chain ID was not set")]
    ChainIdNotSet,
    /// Could not create the data directory.
    #[error("could not create directory: {0}")]
    CreateDirError(std::io::Error),
    /// No stderr was captured from the child process.
    #[error("no stderr was captured from the process")]
    NoStderr,
    /// Timed out waiting for node to start.
    #[error("timed out waiting for node to spawn; is it installed?")]
    Timeout,
    /// Encountered a fatal error.
    #[error("fatal error: {0}")]
    Fatal(String),
    /// A line could not be read from the node stderr.
    #[error("could not read line from node stderr: {0}")]
    ReadLineError(std::io::Error),
    /// Genesis error
    #[error("genesis error occurred: {0}")]
    GenesisError(String),
    /// Node init error
    #[error("node init error occurred")]
    InitError,
    /// Spawn node error
    #[error("could not spawn node: {0}")]
    SpawnError(std::io::Error),
    /// Wait error
    #[error("could not wait for node to exit: {0}")]
    WaitError(std::io::Error),
}

/// Whether or not node is in `dev` mode and configuration options that depend on the mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeMode {
    /// Options that can be set in dev mode
    Dev(DevOptions),
    /// Options that cannot be set in dev mode
    NonDev(PrivateNetOptions),
}

/// Configuration options that can be set in dev mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DevOptions {
    /// The interval at which the dev chain will mine new blocks.
    pub block_time: Option<u64>,
}

/// Configuration options that cannot be set in dev mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateNetOptions {
    /// The p2p port to use.
    pub p2p_port: Option<u16>,

    /// Whether or not peer discovery is enabled.
    pub discovery: bool,
}

impl Default for NodeMode {
    fn default() -> Self {
        Self::Dev(Default::default())
    }
}

impl Default for PrivateNetOptions {
    fn default() -> Self {
        Self { p2p_port: None, discovery: true }
    }
}
