//! Utilities for launching an Anvil instance.

use alloy_primitives::{hex, Address};
use k256::{ecdsa::SigningKey, SecretKey as K256SecretKey};
use std::{
    io::{BufRead, BufReader},
    net::SocketAddr,
    path::PathBuf,
    process::{Child, Command},
    str::FromStr,
    time::{Duration, Instant},
};
use thiserror::Error;
use url::Url;

/// How long we will wait for anvil to indicate that it is ready.
const ANVIL_STARTUP_TIMEOUT_MILLIS: u64 = 10_000;

/// An anvil CLI instance. Will close the instance when dropped.
///
/// Construct this using [`Anvil`].
#[derive(Debug)]
pub struct AnvilInstance {
    child: Child,
    private_keys: Vec<K256SecretKey>,
    addresses: Vec<Address>,
    port: u16,
    chain_id: Option<u64>,
}

impl AnvilInstance {
    /// Returns a reference to the child process.
    pub fn child(&self) -> &Child {
        &self.child
    }

    /// Returns a mutable reference to the child process.
    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Returns the private keys used to instantiate this instance
    pub fn keys(&self) -> &[K256SecretKey] {
        &self.private_keys
    }

    /// Returns the addresses used to instantiate this instance
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    /// Returns the port of this instance
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the chain of the anvil instance
    pub fn chain_id(&self) -> u64 {
        const ANVIL_HARDHAT_CHAIN_ID: u64 = 31_337;
        self.chain_id.unwrap_or(ANVIL_HARDHAT_CHAIN_ID)
    }

    /// Returns the HTTP endpoint of this instance
    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Returns the Websocket endpoint of this instance
    pub fn ws_endpoint(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }

    /// Returns the HTTP endpoint url of this instance
    pub fn endpoint_url(&self) -> Url {
        Url::parse(&self.endpoint()).unwrap()
    }

    /// Returns the Websocket endpoint url of this instance
    pub fn ws_endpoint_url(&self) -> Url {
        Url::parse(&self.ws_endpoint()).unwrap()
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        self.child.kill().expect("could not kill anvil");
    }
}

/// Errors that can occur when working with the [`Anvil`].
#[derive(Debug, Error)]
pub enum AnvilError {
    /// Spawning the anvil process failed.
    #[error("could not start anvil: {0}")]
    SpawnError(std::io::Error),

    /// Timed out waiting for a message from anvil's stderr.
    #[error("timed out waiting for anvil to spawn; is anvil installed?")]
    Timeout,

    /// A line could not be read from the geth stderr.
    #[error("could not read line from anvil stderr: {0}")]
    ReadLineError(std::io::Error),

    /// The child anvil process's stderr was not captured.
    #[error("could not get stderr for anvil child process")]
    NoStderr,

    /// The private key could not be parsed.
    #[error("could not parse private key")]
    ParsePrivateKeyError,

    /// An error occurred while deserializing a private key.
    #[error("could not deserialize private key from bytes")]
    DeserializePrivateKeyError,

    /// An error occurred while parsing a hex string.
    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),
}

/// Builder for launching `anvil`.
///
/// # Panics
///
/// If `spawn` is called without `anvil` being available in the user's $PATH
///
/// # Example
///
/// ```no_run
/// use alloy_node_bindings::Anvil;
///
/// let port = 8545u16;
/// let url = format!("http://localhost:{}", port).to_string();
///
/// let anvil = Anvil::new()
///     .port(port)
///     .mnemonic("abstract vacuum mammal awkward pudding scene penalty purchase dinner depart evoke puzzle")
///     .spawn();
///
/// drop(anvil); // this will kill the instance
/// ```
#[derive(Clone, Debug, Default)]
#[must_use = "This Builder struct does nothing unless it is `spawn`ed"]
pub struct Anvil {
    program: Option<PathBuf>,
    port: Option<u16>,
    // If the block_time is an integer, f64::to_string() will output without a decimal point
    // which allows this to be backwards compatible.
    block_time: Option<f64>,
    chain_id: Option<u64>,
    mnemonic: Option<String>,
    fork: Option<String>,
    fork_block_number: Option<u64>,
    args: Vec<String>,
    timeout: Option<u64>,
}

impl Anvil {
    /// Creates an empty Anvil builder.
    /// The default port is 8545. The mnemonic is chosen randomly.
    ///
    /// # Example
    ///
    /// ```
    /// # use alloy_node_bindings::Anvil;
    /// fn a() {
    ///  let anvil = Anvil::default().spawn();
    ///
    ///  println!("Anvil running at `{}`", anvil.endpoint());
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an Anvil builder which will execute `anvil` at the given path.
    ///
    /// # Example
    ///
    /// ```
    /// # use alloy_node_bindings::Anvil;
    /// fn a() {
    ///  let anvil = Anvil::at("~/.foundry/bin/anvil").spawn();
    ///
    ///  println!("Anvil running at `{}`", anvil.endpoint());
    /// # }
    /// ```
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self::new().path(path)
    }

    /// Sets the `path` to the `anvil` cli
    ///
    /// By default, it's expected that `anvil` is in `$PATH`, see also
    /// [`std::process::Command::new()`]
    pub fn path<T: Into<PathBuf>>(mut self, path: T) -> Self {
        self.program = Some(path.into());
        self
    }

    /// Sets the port which will be used when the `anvil` instance is launched.
    pub fn port<T: Into<u16>>(mut self, port: T) -> Self {
        self.port = Some(port.into());
        self
    }

    /// Sets the chain_id the `anvil` instance will use.
    pub fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Sets the mnemonic which will be used when the `anvil` instance is launched.
    pub fn mnemonic<T: Into<String>>(mut self, mnemonic: T) -> Self {
        self.mnemonic = Some(mnemonic.into());
        self
    }

    /// Sets the block-time in seconds which will be used when the `anvil` instance is launched.
    pub fn block_time(mut self, block_time: u64) -> Self {
        self.block_time = Some(block_time as f64);
        self
    }

    /// Sets the block-time in sub-seconds which will be used when the `anvil` instance is launched.
    /// Older versions of `anvil` do not support sub-second block times.
    pub fn block_time_f64(mut self, block_time: f64) -> Self {
        self.block_time = Some(block_time);
        self
    }

    /// Sets the `fork-block-number` which will be used in addition to [`Self::fork`].
    ///
    /// **Note:** if set, then this requires `fork` to be set as well
    pub fn fork_block_number(mut self, fork_block_number: u64) -> Self {
        self.fork_block_number = Some(fork_block_number);
        self
    }

    /// Sets the `fork` argument to fork from another currently running Ethereum client
    /// at a given block. Input should be the HTTP location and port of the other client,
    /// e.g. `http://localhost:8545`. You can optionally specify the block to fork from
    /// using an @ sign: `http://localhost:8545@1599200`
    pub fn fork<T: Into<String>>(mut self, fork: T) -> Self {
        self.fork = Some(fork.into());
        self
    }

    /// Adds an argument to pass to the `anvil`.
    pub fn arg<T: Into<String>>(mut self, arg: T) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds multiple arguments to pass to the `anvil`.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self = self.arg(arg);
        }
        self
    }

    /// Sets the timeout which will be used when the `anvil` instance is launched.
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Consumes the builder and spawns `anvil`.
    ///
    /// # Panics
    ///
    /// If spawning the instance fails at any point.
    #[track_caller]
    pub fn spawn(self) -> AnvilInstance {
        self.try_spawn().unwrap()
    }

    /// Consumes the builder and spawns `anvil`. If spawning fails, returns an error.
    pub fn try_spawn(self) -> Result<AnvilInstance, AnvilError> {
        let mut cmd = if let Some(ref prg) = self.program {
            Command::new(prg)
        } else {
            Command::new("anvil")
        };
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::inherit());
        let mut port = self.port.unwrap_or_default();
        cmd.arg("-p").arg(port.to_string());

        if let Some(mnemonic) = self.mnemonic {
            cmd.arg("-m").arg(mnemonic);
        }

        if let Some(chain_id) = self.chain_id {
            cmd.arg("--chain-id").arg(chain_id.to_string());
        }

        if let Some(block_time) = self.block_time {
            cmd.arg("-b").arg(block_time.to_string());
        }

        if let Some(fork) = self.fork {
            cmd.arg("-f").arg(fork);
        }

        if let Some(fork_block_number) = self.fork_block_number {
            cmd.arg("--fork-block-number").arg(fork_block_number.to_string());
        }

        cmd.args(self.args);

        let mut child = cmd.spawn().map_err(AnvilError::SpawnError)?;

        let stdout = child.stdout.as_mut().ok_or(AnvilError::NoStderr)?;

        let start = Instant::now();
        let mut reader = BufReader::new(stdout);

        let mut private_keys = Vec::new();
        let mut addresses = Vec::new();
        let mut is_private_key = false;
        let mut chain_id = None;
        loop {
            if start + Duration::from_millis(self.timeout.unwrap_or(ANVIL_STARTUP_TIMEOUT_MILLIS))
                <= Instant::now()
            {
                return Err(AnvilError::Timeout);
            }

            let mut line = String::new();
            reader.read_line(&mut line).map_err(AnvilError::ReadLineError)?;
            trace!(target: "anvil", line);
            if let Some(addr) = line.strip_prefix("Listening on") {
                // <Listening on 127.0.0.1:8545>
                // parse the actual port
                if let Ok(addr) = SocketAddr::from_str(addr.trim()) {
                    port = addr.port();
                }
                break;
            }

            if line.starts_with("Private Keys") {
                is_private_key = true;
            }

            if is_private_key && line.starts_with('(') {
                let key_str =
                    line.split("0x").last().ok_or(AnvilError::ParsePrivateKeyError)?.trim();
                let key_hex = hex::decode(key_str).map_err(AnvilError::FromHexError)?;
                let key = K256SecretKey::from_bytes((&key_hex[..]).into())
                    .map_err(|_| AnvilError::DeserializePrivateKeyError)?;
                addresses.push(Address::from_public_key(SigningKey::from(&key).verifying_key()));
                private_keys.push(key);
            }

            if let Some(start_chain_id) = line.find("Chain ID:") {
                let rest = &line[start_chain_id + "Chain ID:".len()..];
                if let Ok(chain) = rest.split_whitespace().next().unwrap_or("").parse::<u64>() {
                    chain_id = Some(chain);
                };
            }
        }

        Ok(AnvilInstance {
            child,
            private_keys,
            addresses,
            port,
            chain_id: self.chain_id.or(chain_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_launch_anvil() {
        let _ = Anvil::new().spawn();
    }

    #[test]
    fn can_launch_anvil_with_more_accounts() {
        let _ = Anvil::new().arg("--accounts").arg("20").spawn();
    }

    #[test]
    fn assert_block_time_is_natural_number() {
        //This test is to ensure that older versions of anvil are supported
        //even though the block time is a f64, it should be passed as a whole number
        let anvil = Anvil::new().block_time(12);
        assert_eq!(anvil.block_time.unwrap().to_string(), "12");
        let _ = anvil.spawn();
    }

    #[test]
    fn can_launch_anvil_with_sub_seconds_block_time() {
        let _ = Anvil::new().block_time_f64(0.5).spawn();
    }

    #[test]
    fn assert_chain_id() {
        let anvil = Anvil::new().fork("https://rpc.ankr.com/eth").spawn();
        assert_eq!(anvil.chain_id(), 1);
    }

    #[test]
    fn assert_chain_id_without_rpc() {
        let anvil = Anvil::new().spawn();
        assert_eq!(anvil.chain_id(), 31337);
    }
}
