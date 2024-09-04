//! Utilities for launching a Reth dev-mode instance.

use crate::{extract_endpoint, NodeError, NodeInstanceError, NODE_STARTUP_TIMEOUT};
use alloy_genesis::Genesis;
use std::{
    fs::create_dir,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, ChildStdout, Command, Stdio},
    time::Instant,
};
use url::Url;

/// The exposed APIs
const API: &str = "eth,net,web3,txpool,trace,rpc,reth,ots,admin,debug";

/// The reth command
const RETH: &str = "reth";

/// A reth instance. Will close the instance when dropped.
///
/// Construct this using [`Reth`].
#[derive(Debug)]
pub struct RethInstance {
    pid: Child,
    http_port: u16,
    ws_port: u16,
    auth_port: Option<u16>,
    p2p_port: Option<u16>,
    ipc: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    genesis: Option<Genesis>,
}

impl RethInstance {
    /// Returns the HTTP port of this instance
    pub const fn http_port(&self) -> u16 {
        self.http_port
    }

    /// Returns the WS port of this instance
    pub const fn ws_port(&self) -> u16 {
        self.ws_port
    }

    /// Returns the auth port of this instance
    pub const fn auth_port(&self) -> Option<u16> {
        self.auth_port
    }

    /// Returns the p2p port of this instance
    /// If discovery is disabled, this will be `None`
    pub const fn p2p_port(&self) -> Option<u16> {
        self.p2p_port
    }

    /// Returns the HTTP endpoint of this instance
    #[doc(alias = "http_endpoint")]
    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.http_port)
    }

    /// Returns the Websocket endpoint of this instance
    pub fn ws_endpoint(&self) -> String {
        format!("ws://localhost:{}", self.ws_port)
    }

    /// Returns the IPC endpoint of this instance
    pub fn ipc_endpoint(&self) -> String {
        self.ipc.clone().map_or_else(|| "reth.ipc".to_string(), |ipc| ipc.display().to_string())
    }

    /// Returns the HTTP endpoint url of this instance
    #[doc(alias = "http_endpoint_url")]
    pub fn endpoint_url(&self) -> Url {
        Url::parse(&self.endpoint()).unwrap()
    }

    /// Returns the Websocket endpoint url of this instance
    pub fn ws_endpoint_url(&self) -> Url {
        Url::parse(&self.ws_endpoint()).unwrap()
    }

    /// Returns the path to this instances' data directory
    pub const fn data_dir(&self) -> &Option<PathBuf> {
        &self.data_dir
    }

    /// Returns the genesis configuration used to configure this instance
    pub const fn genesis(&self) -> &Option<Genesis> {
        &self.genesis
    }

    /// Takes the stdout contained in the child process.
    ///
    /// This leaves a `None` in its place, so calling methods that require a stdout to be present
    /// will fail if called after this.
    pub fn stdout(&mut self) -> Result<ChildStdout, NodeInstanceError> {
        self.pid.stdout.take().ok_or(NodeInstanceError::NoStdout)
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
/// let reth = Reth::new().instance(0).block_time("12sec").spawn();
///
/// drop(reth); // this will kill the instance
/// ```
#[derive(Clone, Debug, Default)]
#[must_use = "This Builder struct does nothing unless it is `spawn`ed"]
pub struct Reth {
    dev: bool,
    block_time: Option<String>,
    instance: u16,
    discovery_enabled: bool,
    program: Option<PathBuf>,
    ipc_path: Option<PathBuf>,
    ipc_enabled: bool,
    data_dir: Option<PathBuf>,
    chain_or_path: Option<String>,
    genesis: Option<Genesis>,
}

impl Reth {
    /// Creates an empty Reth builder.
    ///
    /// The mnemonic is chosen randomly.
    pub const fn new() -> Self {
        Self {
            dev: false,
            block_time: None,
            instance: 0,
            discovery_enabled: true,
            program: None,
            ipc_path: None,
            ipc_enabled: false,
            data_dir: None,
            chain_or_path: None,
            genesis: None,
        }
    }

    /// Creates a Reth builder which will execute `reth` at the given path.
    ///
    /// # Example
    ///
    /// ```
    /// use alloy_node_bindings::Reth;
    /// # fn a() {
    /// let reth = Reth::at("../reth/target/release/reth").spawn();
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

    /// Enable `dev` mode for the reth instance.
    pub const fn dev(mut self) -> Self {
        self.dev = true;
        self
    }

    /// Sets the block time for the reth instance.
    /// Parses strings using <https://docs.rs/humantime/latest/humantime/fn.parse_duration.html>
    /// This is only used if `dev` mode is enabled.
    pub fn block_time(mut self, block_time: &str) -> Self {
        self.block_time = Some(block_time.to_string());
        self
    }

    /// Disables discovery for the reth instance.
    pub const fn disable_discovery(mut self) -> Self {
        self.discovery_enabled = false;
        self
    }

    /// Sets the chain id for the reth instance.
    pub fn chain_or_path(mut self, chain_or_path: &str) -> Self {
        self.chain_or_path = Some(chain_or_path.to_string());
        self
    }

    /// Enable IPC for the reth instance.
    pub const fn enable_ipc(mut self) -> Self {
        self.ipc_enabled = true;
        self
    }

    /// Sets the instance number for the reth instance.
    pub const fn instance(mut self, instance: u16) -> Self {
        self.instance = instance;
        self
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
        // `reth` uses stdout for its logs
        cmd.stdout(Stdio::piped());

        // Use Reth's `node` subcommand.
        cmd.arg("node");

        // If the `dev` flag is set, enable it.
        if self.dev {
            // Enable the dev mode.
            // This mode uses a local proof-of-authority consensus engine with either fixed block
            // times or automatically mined blocks.
            // Disables network discovery and enables local http server.
            // Prefunds 20 accounts derived by mnemonic "test test test test test test test test
            // test test test junk" with 10 000 ETH each.
            cmd.arg("--dev");

            // If the block time is set, use it.
            if let Some(block_time) = self.block_time {
                cmd.arg("--dev.block-time").arg(block_time);
            }
        }

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

        // Configure the IPC path if it is set.
        if let Some(ipc) = &self.ipc_path {
            cmd.arg("--ipcpath").arg(ipc);
        }

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

        if !self.discovery_enabled {
            cmd.arg("--disable-discovery");
            cmd.arg("--no-persist-peers");
        } else {
            // Verbosity is required to read the P2P port from the logs.
            cmd.arg("--verbosity").arg("-vvv");
        }

        if let Some(chain_or_path) = self.chain_or_path {
            cmd.arg("--chain").arg(chain_or_path);
        }

        // Disable color output to make parsing logs easier.
        cmd.arg("--color").arg("never");

        dbg!(&cmd);

        let mut child = cmd.spawn().map_err(NodeError::SpawnError)?;

        let stdout = child.stdout.take().ok_or(NodeError::NoStdout)?;

        let start = Instant::now();
        let mut reader = BufReader::new(stdout);

        let mut http_port = 0;
        let mut ws_port = 0;
        let mut auth_port = 0;
        let mut p2p_port = 0;

        let mut ports_started = false;
        let mut p2p_started = !self.discovery_enabled;

        let mut should_exit = false;

        loop {
            if start + NODE_STARTUP_TIMEOUT <= Instant::now() {
                let _ = child.kill();
                return Err(NodeError::Timeout);
            }

            let mut line = String::with_capacity(200);
            reader.read_line(&mut line).map_err(NodeError::ReadLineError)?;

            dbg!(&line);

            if line.contains("RPC HTTP server started") {
                if let Some(addr) = extract_endpoint("url=", &line) {
                    http_port = addr.port();
                }
            }

            if line.contains("RPC WS server started") {
                if let Some(addr) = extract_endpoint("url=", &line) {
                    ws_port = addr.port();
                }
            }

            if line.contains("RPC auth server started") {
                if let Some(addr) = extract_endpoint("url=", &line) {
                    auth_port = addr.port();
                }
            }

            // Encountered a critical error, exit early.
            if line.contains("ERROR") {
                should_exit = true;
            }

            if http_port != 0 && ws_port != 0 && auth_port != 0 {
                ports_started = true;
            }

            if self.discovery_enabled {
                if line.contains("Updated local ENR") {
                    if let Some(port) = extract_endpoint("IpV4 UDP Socket", &line) {
                        p2p_port = port.port();
                        p2p_started = true;
                    }
                }
            } else {
                p2p_started = true;
            }

            // If all ports have started we are ready to be queried.
            if ports_started && p2p_started {
                break;
            }
        }

        if should_exit {
            let _ = child.kill();
            return Err(NodeError::Fatal("Encountered a critical error".to_string()));
        }

        child.stdout = Some(reader.into_inner());

        Ok(RethInstance {
            pid: child,
            http_port,
            ws_port,
            p2p_port: if p2p_port != 0 { Some(p2p_port) } else { None },
            ipc: self.ipc_path,
            data_dir: self.data_dir,
            auth_port: Some(auth_port),
            genesis: self.genesis,
        })
    }
}

// These tests should use a different datadir for each `Reth` spawned
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn can_launch_reth() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new().data_dir(temp_dir_path).spawn();

            assert_ports(&reth, false);
        });
    }

    #[test]
    fn can_launch_reth_sepolia() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new().chain_or_path("sepolia").data_dir(temp_dir_path).spawn();

            assert_ports(&reth, false);
        });
    }

    #[test]
    fn can_launch_reth_dev() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir_path).spawn();

            assert_ports(&reth, true);
        });
    }

    #[test]
    fn can_launch_reth_dev_custom_genesis() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new()
                .dev()
                .disable_discovery()
                .data_dir(temp_dir_path)
                .genesis(Genesis::default())
                .spawn();

            assert_ports(&reth, true);
        });
    }

    #[test]
    fn can_launch_reth_dev_custom_blocktime() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new()
                .dev()
                .disable_discovery()
                .block_time("1sec")
                .data_dir(temp_dir_path)
                .spawn();

            assert_ports(&reth, true);
        });
    }

    #[test]
    fn can_launch_reth_p2p_instance1() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new().instance(1).data_dir(temp_dir_path).spawn();

            assert_eq!(reth.http_port(), 8545);
            assert_eq!(reth.ws_port(), 8546);
            assert_eq!(reth.auth_port(), Some(8551));
            assert_eq!(reth.p2p_port(), Some(30303));
        });
    }

    #[test]
    fn can_launch_reth_p2p_instance2() {
        run_with_tempdir(|temp_dir_path| {
            let reth = Reth::new().instance(2).data_dir(temp_dir_path).spawn();

            assert_eq!(reth.http_port(), 8544);
            assert_eq!(reth.ws_port(), 8548);
            assert_eq!(reth.auth_port(), Some(8651));
            assert_eq!(reth.p2p_port(), Some(30304));
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

    fn assert_ports(reth: &RethInstance, dev: bool) {
        assert_eq!(reth.http_port(), 8545);
        assert_eq!(reth.ws_port(), 8546);
        assert_eq!(reth.auth_port(), Some(8551));

        if dev {
            assert_eq!(reth.p2p_port(), None);
        } else {
            assert_eq!(reth.p2p_port(), Some(30303));
        }
    }
}
