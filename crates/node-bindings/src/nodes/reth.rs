//! Utilities for launching a Reth dev-mode instance.

use crate::{NodeConfig, NodeInstance, NodeInstanceError};
use std::process::Child;

/// The reth command
const RETH: &str = "reth";

/// The Reth instance.
pub struct RethInstance {
    /// The configuration of the node.
    config: NodeConfig,
    /// The child process of the node.
    pid: Child,
}

impl RethInstance {
    /// Creates a new Reth instance.
    pub fn new(config: NodeConfig, pid: Child) -> Self {
        Self { config, pid }
    }
}

impl NodeInstance for RethInstance {
    fn config(&self) -> &NodeConfig {
        &self.config
    }

    fn pid(&mut self) -> &mut Child {
        &mut self.pid
    }

    fn ipc_endpoint(&self) -> String {
        self.config().ipc.as_ref().map(|ipc| ipc.display().to_string()).to_owned().unwrap()
    }

    fn wait_to_add_peer(&mut self, _id: &str) -> Result<(), NodeInstanceError> {
        unimplemented!()
    }
}

impl Drop for RethInstance {
    fn drop(&mut self) {
        self.pid.kill().expect("could not kill reth");
    }
}
