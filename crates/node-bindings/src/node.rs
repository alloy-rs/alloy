//! Node-related types and constants.

use std::time::Duration;
use thiserror::Error;

/// How long we will wait for the node to indicate that it is ready.
pub const NODE_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for waiting for the node to add a peer.
pub const NODE_DIAL_LOOP_TIMEOUT: Duration = Duration::from_secs(20);

/// Errors that can occur when working with a node instance.
#[derive(Debug)]
pub enum NodeInstanceError {
    /// Timed out waiting for a message from node's stderr.
    Timeout(String),

    /// A line could not be read from the node's stderr.
    ReadLineError(std::io::Error),

    /// The child node process's stderr was not captured.
    NoStderr,
}

/// Errors that can occur when working with the node.
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
    /// Timed out waiting for the node to start.
    #[error("timed out waiting for node to spawn; is the node binary installed?")]
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

    /// Clique private key error
    #[error("clique address error: {0}")]
    CliqueAddressError(String),
}

/// Whether or not node is in `dev` mode and configuration options that depend on the mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeMode {
    /// Options that can be set in dev mode
    Dev(DevOptions),
    /// Options that cannot be set in dev mode
    NonDev(PrivateNetOptions),
}

impl Default for NodeMode {
    fn default() -> Self {
        Self::Dev(Default::default())
    }
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

impl Default for PrivateNetOptions {
    fn default() -> Self {
        Self { p2p_port: None, discovery: true }
    }
}
