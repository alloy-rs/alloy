//! Utilities for launching a Reth dev-mode instance.

use crate::{utils::extract_endpoint, NodeError, NODE_STARTUP_TIMEOUT};
use alloy_genesis::Genesis;
use alloy_primitives::{hex, Address};
use k256::SecretKey;
use rand::Rng;
use std::{
    fs::create_dir,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, ChildStdout, Command, Stdio},
    str::FromStr,
    time::Instant,
};
use url::Url;

/// The exposed APIs
const API: &str = "eth,net,web3,txpool,trace,rpc,reth,ots,admin,debug";

/// The reth command
const RETH: &str = "reth";

/// A Reth instance. Will close the instance when dropped.
///
/// Construct this using [`Reth`].
#[derive(Debug)]
pub struct RethInstance {
    pid: Child,
    instance: u16,
    private_keys: Vec<SecretKey>,
    addresses: Vec<Address>,
    http_port: u16,
    ws_port: u16,
    auth_port: Option<u16>,
    p2p_port: Option<u16>,
    ipc: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    genesis: Option<Genesis>,
}

impl RethInstance {
    /// Returns the instance number of this instance.
    pub const fn instance(&self) -> u16 {
        self.instance
    }

    /// Returns the private keys used to instantiate this instance.
    /// Only available in dev mode.
    pub fn keys(&self) -> &[SecretKey] {
        &self.private_keys
    }

    /// Returns the addresses used to instantiate this instance
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    /// Returns the HTTP port of this instance.
    pub const fn http_port(&self) -> u16 {
        self.http_port
    }

    /// Returns the WS port of this instance.
    pub const fn ws_port(&self) -> u16 {
        self.ws_port
    }

    /// Returns the auth port of this instance.
    pub const fn auth_port(&self) -> Option<u16> {
        self.auth_port
    }

    /// Returns the p2p port of this instance.
    /// If discovery is disabled, this will be `None`.
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
    pub fn stdout(&mut self) -> Result<ChildStdout, NodeError> {
        self.pid.stdout.take().ok_or(NodeError::NoStdout)
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
/// let reth = Reth::new().instance(1).block_time("12sec").spawn();
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

    /// Enable `dev` mode for the Reth instance.
    pub const fn dev(mut self) -> Self {
        self.dev = true;
        self
    }

    /// Sets the block time for the Reth instance.
    /// Parses strings using <https://docs.rs/humantime/latest/humantime/fn.parse_duration.html>
    /// This is only used if `dev` mode is enabled.
    pub fn block_time(mut self, block_time: &str) -> Self {
        self.block_time = Some(block_time.to_string());
        self
    }

    /// Disables discovery for the Reth instance.
    pub const fn disable_discovery(mut self) -> Self {
        self.discovery_enabled = false;
        self
    }

    /// Sets the chain id for the Reth instance.
    pub fn chain_or_path(mut self, chain_or_path: &str) -> Self {
        self.chain_or_path = Some(chain_or_path.to_string());
        self
    }

    /// Enable IPC for the Reth instance.
    pub const fn enable_ipc(mut self) -> Self {
        self.ipc_enabled = true;
        self
    }

    /// Sets the instance number for the Reth instance.
    pub const fn instance(mut self, instance: u16) -> Self {
        self.instance = instance;
        self
    }

    /// Sets the instance number to a random number to reduce flakiness.
    pub fn random_instance(mut self) -> Self {
        // Reth limits the number of instances to 200.
        self.instance = rand::thread_rng().gen_range(0..200);
        self
    }

    /// Sets the IPC path for the socket.
    pub fn ipc_path<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.ipc_path = Some(path.into());
        self
    }

    /// Sets the data directory for Reth.
    pub fn data_dir<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.data_dir = Some(path.into());
        self
    }

    /// Sets the `genesis.json` for the Reth instance.
    ///
    /// If this is set, Reth will be initialized with `reth init` and the `--datadir` option will be
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
        // Reth uses stdout for its logs
        cmd.stdout(Stdio::piped());

        // Use Reth's `node` subcommand.
        cmd.arg("node");

        // If the `dev` flag is set, enable it.
        let mut addresses = Vec::new();
        let mut private_keys = Vec::new();

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

            // Set the default keys and addresses that are used in dev mode.
            addresses = [
                "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
                "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
                "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
                "0x90F79bf6EB2c4f870365E785982E1f101E93b906",
                "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65",
                "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc",
                "0x976EA74026E726554dB657fA54763abd0C3a0aa9",
                "0x14dC79964da2C08b23698B3D3cc7Ca32193d9955",
                "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f",
                "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720",
            ]
            .iter()
            .map(|s| Address::from_str(s).unwrap())
            .collect::<Vec<Address>>();

            private_keys = [
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
                "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
                "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
                "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
                "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
                "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
                "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
                "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
                "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
                "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
            ]
            .iter()
            .map(|s| {
                let key_hex = hex::decode(s).unwrap();
                SecretKey::from_bytes((&key_hex[..]).into()).unwrap()
            })
            .collect::<Vec<SecretKey>>();
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

        loop {
            if start + NODE_STARTUP_TIMEOUT <= Instant::now() {
                let _ = child.kill();
                return Err(NodeError::Timeout);
            }

            let mut line = String::with_capacity(120);
            reader.read_line(&mut line).map_err(NodeError::ReadLineError)?;

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
                let _ = child.kill();
                return Err(NodeError::Fatal(line));
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

        child.stdout = Some(reader.into_inner());

        Ok(RethInstance {
            pid: child,
            instance: self.instance,
            private_keys,
            addresses,
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

// These tests should use a different datadir for each `reth` instance spawned.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_with_tempdir_sync;

    const DISCOVERY_PORT: u16 = 30303;
    const AUTH_PORT: u16 = 8551;
    const HTTP_RPC_PORT: u16 = 8545;
    const WS_RPC_PORT: u16 = 8546;

    #[test]
    #[cfg(not(windows))]
    fn can_launch_reth() {
        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new().random_instance().data_dir(temp_dir_path).spawn();

            assert_ports(&reth, false);
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn can_launch_reth_sepolia() {
        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new()
                .random_instance()
                .chain_or_path("sepolia")
                .data_dir(temp_dir_path)
                .spawn();

            assert_ports(&reth, false);
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn can_launch_reth_dev() {
        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new()
                .random_instance()
                .dev()
                .disable_discovery()
                .data_dir(temp_dir_path)
                .spawn();

            assert_ports(&reth, true);
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn can_launch_reth_dev_custom_genesis() {
        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new()
                .random_instance()
                .dev()
                .disable_discovery()
                .data_dir(temp_dir_path)
                .genesis(Genesis::default())
                .spawn();

            assert_ports(&reth, true);
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn can_launch_reth_dev_custom_blocktime() {
        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new()
                .random_instance()
                .dev()
                .disable_discovery()
                .block_time("1sec")
                .data_dir(temp_dir_path)
                .spawn();

            assert_ports(&reth, true);
        });
    }

    // Asserts that the ports are set correctly for the given Reth instance.
    fn assert_ports(reth: &RethInstance, dev: bool) {
        // Changes to the following port numbers for each instance:
        // - `DISCOVERY_PORT`: default + `instance` - 1
        // - `AUTH_PORT`: default + `instance` * 100 - 100
        // - `HTTP_RPC_PORT`: default - `instance` + 1
        // - `WS_RPC_PORT`: default + `instance` * 2 - 2
        assert_eq!(reth.http_port(), HTTP_RPC_PORT - reth.instance + 1);
        assert_eq!(reth.ws_port(), WS_RPC_PORT + reth.instance * 2 - 2);
        assert_eq!(reth.auth_port(), Some(AUTH_PORT + reth.instance * 100 - 100));
        assert_eq!(
            reth.p2p_port(),
            if dev { None } else { Some(DISCOVERY_PORT + reth.instance - 1) }
        );
    }
}
