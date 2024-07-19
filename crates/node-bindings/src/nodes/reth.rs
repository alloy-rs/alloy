//! Utilities for launching a Reth dev-mode instance.

use crate::{
    extract_endpoint, DevOptions, NodeConfig, NodeError, NodeInstance, NodeInstanceError, NodeMode,
    PrivateNetOptions, NODE_STARTUP_TIMEOUT,
};
use alloy_genesis::Genesis;
use std::{
    fs::{create_dir, File},
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Instant,
};

/// The exposed APIs
const API: &str = "eth,net,web3,txpool,trace,rpc,reth,ots,admin,debug";

/// The reth command
const RETH: &str = "reth";

/// The Reth instance.
#[derive(Debug)]
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

    fn p2p_port(&self) -> Option<u16> {
        unimplemented!()
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

/// Builder for launching `reth`.
///
/// # Panics
///
/// If `spawn` is called without `reth` being available in the user's $PATH
///
/// # Example
///
/// ```no_run
/// use alloy_node_bindings::Reth;
///
/// let port = 8545u16;
/// let url = format!("http://localhost:{}", port).to_string();
///
/// let reth = Reth::new().port(port).block_time(5000u64).spawn();
///
/// drop(reth); // this will kill the instance
/// ```
#[derive(Clone, Debug, Default)]
#[must_use = "This Builder struct does nothing unless it is `spawn`ed"]
pub struct Reth {
    instance: u16,
    program: Option<PathBuf>,
    port: Option<u16>,
    authrpc_port: Option<u16>,
    ipc_path: Option<PathBuf>,
    ipc_enabled: bool,
    data_dir: Option<PathBuf>,
    chain_id: Option<u64>,
    insecure_unlock: bool,
    genesis: Option<Genesis>,
    mode: NodeMode,
}

impl Reth {
    /// Creates an empty Reth builder.
    ///
    /// The mnemonic is chosen randomly.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a Reth builder which will execute `reth` at the given path.
    ///
    /// # Example
    ///
    /// ```
    /// use alloy_node_bindings::Reth;
    /// # fn a() {
    /// let reth = Reth::at("../go-ethereum/build/bin/reth").spawn();
    ///
    /// println!("Reth running at `{}`", reth.endpoint());
    /// # }
    /// ```
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self::new().path(path)
    }

    /// Sets the `path` to the `reth` executable
    ///
    /// By default, it's expected that `reth` is in `$PATH`, see also
    /// [`std::process::Command::new()`]
    pub fn path<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.program = Some(path.into());
        self
    }

    /// Sets the port which will be used when the `reth-cli` instance is launched.
    ///
    /// If port is 0 then the OS will choose a random port.
    /// [RethInstance::port] will return the port that was chosen.
    pub fn port<T: Into<u16>>(mut self, port: T) -> Self {
        self.port = Some(port.into());
        self
    }

    /// Sets the block-time which will be used when the `reth-cli` instance is launched.
    ///
    /// This will put the reth instance in `dev` mode, discarding any previously set options that
    /// cannot be used in dev mode.
    pub const fn block_time(mut self, block_time: u64) -> Self {
        self.mode = NodeMode::Dev(DevOptions { block_time: Some(block_time) });
        self
    }

    /// Sets the chain id for the reth instance.
    pub const fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Allow reth to unlock accounts when rpc apis are open.
    pub const fn insecure_unlock(mut self) -> Self {
        self.insecure_unlock = true;
        self
    }

    /// Enable IPC for the reth instance.
    pub const fn enable_ipc(mut self) -> Self {
        self.ipc_enabled = true;
        self
    }

    /// Sets the instance number for the reth instance.
    pub fn instance(mut self, instance: u16) -> Self {
        self.instance = instance;
        self
    }

    /// Disable discovery for the reth instance.
    ///
    /// This will put the reth instance into non-dev mode, discarding any previously set dev-mode
    /// options.
    pub fn disable_discovery(mut self) -> Self {
        self.inner_disable_discovery();
        self
    }

    fn inner_disable_discovery(&mut self) {
        match &mut self.mode {
            NodeMode::Dev(_) => {
                self.mode =
                    NodeMode::NonDev(PrivateNetOptions { discovery: false, ..Default::default() })
            }
            NodeMode::NonDev(opts) => opts.discovery = false,
        }
    }

    /// Sets the IPC path for the socket.
    pub fn ipc_path<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.ipc_path = Some(path.into());
        self
    }

    /// Sets the data directory for reth.
    pub fn data_dir<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.data_dir = Some(path.into());
        self
    }

    /// Sets the `genesis.json` for the reth instance.
    ///
    /// If this is set, reth will be initialized with `reth init` and the `--datadir` option will be
    /// set to the same value as `data_dir`.
    ///
    /// This is destructive and will overwrite any existing data in the data directory.
    pub fn genesis(mut self, genesis: Genesis) -> Self {
        self.genesis = Some(genesis);
        self
    }

    /// Sets the port for authenticated RPC connections.
    pub const fn authrpc_port(mut self, port: u16) -> Self {
        self.authrpc_port = Some(port);
        self
    }

    /// Consumes the builder and spawns `reth`.
    ///
    /// # Panics
    ///
    /// If spawning the instance fails at any point.
    #[track_caller]
    pub fn spawn(self) -> RethInstance {
        self.try_spawn().unwrap()
    }

    /// Consumes the builder and spawns `reth`. If spawning fails, returns an error.
    pub fn try_spawn(self) -> Result<RethInstance, NodeError> {
        let bin_path = self
            .program
            .as_ref()
            .map_or_else(|| RETH.as_ref(), |bin| bin.as_os_str())
            .to_os_string();
        let mut cmd = Command::new(&bin_path);
        // reth uses stderr for its logs
        cmd.stderr(Stdio::piped());

        // Use Reth's `node` subcommand.
        cmd.arg("node");

        // If IPC is not enabled on the builder, disable it.
        if !self.ipc_enabled {
            cmd.arg("--ipcdisable");
        }

        // Open the HTTP API.
        cmd.arg("--http");
        cmd.arg("--http.api").arg(API);

        // Open the WS API.
        cmd.arg("--ws");
        cmd.arg("--ws.api").arg(API);

        // Configures the ports of the node to avoid conflicts with the defaults. This is useful for
        // running multiple nodes on the same machine.
        //
        // Changes to the following port numbers:
        // - `DISCOVERY_PORT`: default + `instance` - 1
        // - `AUTH_PORT`: default + `instance` * 100 - 100
        // - `HTTP_RPC_PORT`: default - `instance` + 1
        // - `WS_RPC_PORT`: default + `instance` * 2 - 2
        if self.instance > 0 {
            cmd.arg("--instance").arg(self.instance.to_string());
        }

        if let Some(data_dir) = &self.data_dir {
            cmd.arg("--datadir").arg(data_dir);

            // create the directory if it doesn't exist
            if !data_dir.exists() {
                create_dir(data_dir).map_err(NodeError::CreateDirError)?;
            }
        }

        // Dev mode with custom block time
        match self.mode {
            NodeMode::Dev(DevOptions { block_time }) => {
                cmd.arg("--dev");
                if let Some(block_time) = block_time {
                    cmd.arg("--dev.block-time").arg(block_time.to_string());
                }
            }
            NodeMode::NonDev(PrivateNetOptions { discovery, .. }) => {
                // disable discovery if the flag is set
                if !discovery {
                    cmd.arg("--disable-discovery");
                }
            }
        };

        if let Some(chain_id) = self.chain_id {
            cmd.arg("--chain").arg(chain_id.to_string());
        }

        // debug verbosity is needed to check when peers are added
        cmd.arg("--verbosity").arg("-vvvv");

        if let Some(ipc) = &self.ipc_path {
            cmd.arg("--ipcpath").arg(ipc);
        }

        let mut child = cmd.spawn().map_err(NodeError::SpawnError)?;

        let stderr = child.stderr.ok_or(NodeError::NoStderr)?;

        let start = Instant::now();
        let mut reader = BufReader::new(stderr);

        let mut p2p_started = matches!(self.mode, NodeMode::Dev(_));
        let mut http_started = false;

        let mut port = 0;
        let mut p2p_port = 0;
        let mut auth_port = 0;

        loop {
            if start + NODE_STARTUP_TIMEOUT <= Instant::now() {
                return Err(NodeError::Timeout);
            }

            let mut line = String::with_capacity(120);
            reader.read_line(&mut line).map_err(NodeError::ReadLineError)?;

            if line.contains("RPC auth server started") {
                if let Some(addr) = extract_endpoint("url=", &line) {
                    auth_port = addr.port();
                }
            }

            if line.contains("HTTP server started") {
                // Extracts the address from the output
                if let Some(addr) = extract_endpoint("url=", &line) {
                    // use the actual http port
                    port = addr.port();
                }

                http_started = true;
            }

            if line.contains("opened UDP socket") {
                if let Some(addr) = extract_endpoint("local_addr=", &line) {
                    // use the actual p2p port
                    p2p_port = addr.port();
                }

                p2p_started = true;
            }

            // Encountered an error such as Fatal: Error starting protocol stack: listen tcp
            // 127.0.0.1:8545: bind: address already in use
            if line.contains("ERROR") {
                return Err(NodeError::Fatal(line));
            }

            if p2p_started && http_started {
                break;
            }
        }

        child.stderr = Some(reader.into_inner());

        Ok(RethInstance {
            pid: child,
            config: NodeConfig::default()
                .port(Some(port))
                .p2p_port(Some(p2p_port))
                .auth_port(Some(auth_port))
                .data_dir(self.data_dir)
                .ipc(self.ipc_path)
                .genesis(self.genesis),
        })
    }
}

// These tests should use a different datadir for each `Reth` spawned
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn instance_0() {
        run_with_tempdir(|_| {
            let _reth = Reth::new().disable_discovery().instance(0).spawn();
        });
    }

    /// Allows running tests with a temporary directory, which is cleaned up after the function is
    /// called.
    ///
    /// Helps with tests that spawn a helper instance, which has to be dropped before the temporary
    /// directory is cleaned up.
    #[track_caller]
    fn run_with_tempdir(f: impl Fn(&Path)) {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path();
        f(temp_dir_path);
        #[cfg(not(windows))]
        temp_dir.close().unwrap();
    }
}
