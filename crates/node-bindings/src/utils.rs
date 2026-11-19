//! Utility functions for the node bindings.

use std::{
    borrow::Cow,
    future::Future,
    io::{self, BufRead, BufReader, Read},
    net::{SocketAddr, TcpListener},
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::Child,
    time::{Duration, Instant},
};
use tempfile::TempDir;

/// Reads a node's output line by line.
///
/// A [`BufReader`] reads ahead past the line it returns, so its spare bytes are lost once the
/// handle is handed back to the caller. Output that is handed back is therefore read one byte at
/// a time, leaving every byte after the last line read in the handle.
#[derive(Debug)]
pub(crate) enum LineReader<R> {
    /// Output that is not handed back, read through a buffer.
    Buffered(BufReader<R>),
    /// Output that is handed back, read without reading ahead.
    Unbuffered(R),
}

impl<R: Read> LineReader<R> {
    /// Reads `inner`, without reading ahead when it will be handed back (`keep`).
    pub(crate) fn new(inner: R, keep: bool) -> Self {
        if keep {
            Self::Unbuffered(inner)
        } else {
            Self::Buffered(BufReader::new(inner))
        }
    }

    /// Appends the next line to `buf`, like [`BufRead::read_line`]; returns 0 at EOF.
    pub(crate) fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            Self::Buffered(reader) => reader.read_line(buf),
            Self::Unbuffered(reader) => read_line_unbuffered(reader, buf),
        }
    }

    /// Returns the handle. Only an unbuffered reader keeps every byte not read yet.
    pub(crate) fn into_inner(self) -> R {
        match self {
            Self::Buffered(reader) => reader.into_inner(),
            Self::Unbuffered(reader) => reader,
        }
    }
}

/// Appends the next line of `reader` to `buf`, reading one byte at a time so that nothing past
/// the newline is consumed. Returns the number of bytes read, 0 at EOF.
pub(crate) fn read_line_unbuffered(reader: &mut impl Read, buf: &mut String) -> io::Result<usize> {
    let mut line = Vec::new();
    let mut byte = [0; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
    let read = line.len();
    let line = String::from_utf8(line).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "stream did not contain valid UTF-8")
    })?;
    buf.push_str(&line);
    Ok(read)
}

/// Helper for graceful process shutdown.
pub(crate) struct GracefulShutdown;

impl GracefulShutdown {
    /// Attempts graceful shutdown with SIGTERM, then SIGKILL after timeout.
    pub(crate) fn shutdown(child: &mut Child, timeout_secs: u64, process_name: &str) {
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }

            let timeout = Duration::from_secs(timeout_secs);
            let start = Instant::now();

            while start.elapsed() < timeout {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => break,
                }
            }
        }

        child.kill().unwrap_or_else(|_| panic!("could not kill {}", process_name));
        let _ = child.wait();
    }
}

/// Owns a child until startup succeeds, cleaning it up on every error path.
pub(crate) struct StartupGuard(Option<Child>);

impl StartupGuard {
    pub(crate) const fn new(child: Child) -> Self {
        Self(Some(child))
    }

    pub(crate) fn into_child(mut self) -> Child {
        self.0.take().unwrap()
    }
}

impl Deref for StartupGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl DerefMut for StartupGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

impl Drop for StartupGuard {
    fn drop(&mut self) {
        let Some(child) = &mut self.0 else { return };
        if !matches!(child.try_wait(), Ok(Some(_))) && child.kill().is_ok() {
            // Waiting after a failed kill could block on a process that is still running.
            let _ = child.wait();
        }
    }
}

/// A bit of hack to find an unused TCP port.
///
/// Does not guarantee that the given port is unused after the function exists, just that it was
/// unused before the function started (i.e., it does not reserve a port).
pub(crate) fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to create TCP listener to find unused port");

    let local_addr =
        listener.local_addr().expect("Failed to read TCP listener local_addr to find unused port");
    local_addr.port()
}

/// Extracts the value for the given key from the line of text.
///
/// It supports keys that end with '=' or ': '.
/// For keys end with '=', find value until ' ' is encountered or end of line
/// For keys end with ':', find value until ',' is encountered or end of line
pub(crate) fn extract_value<'a>(key: &str, line: &'a str) -> Option<&'a str> {
    let mut key_equal = Cow::from(key);
    let mut key_colon = Cow::from(key);

    // Prepare both key variants
    if !key_equal.ends_with('=') {
        key_equal = format!("{key}=").into();
    }
    if !key_colon.ends_with(": ") {
        key_colon = format!("{key}: ").into();
    }

    // Try to find the key with '='
    if let Some(pos) = line.find(key_equal.as_ref()) {
        let start = pos + key_equal.len();
        let end = line[start..].find(' ').map(|i| start + i).unwrap_or(line.len());
        return Some(line[start..end].trim());
    }

    // If not found, try to find the key with ': '
    if let Some(pos) = line.find(key_colon.as_ref()) {
        let start = pos + key_colon.len();
        let end = line[start..].find(',').map(|i| start + i).unwrap_or(line.len()); // Assuming comma or end of line
        return Some(line[start..end].trim());
    }

    // If neither variant matches, return None
    None
}

/// Extracts the endpoint from the given line.
pub(crate) fn extract_endpoint(key: &str, line: &str) -> Option<SocketAddr> {
    extract_value(key, line)
        .map(|val| val.trim_start_matches("Some(").trim_end_matches(')'))
        .and_then(|val| val.parse().ok())
}

/// Runs the given closure with a temporary directory.
pub fn run_with_tempdir_sync(prefix: &str, f: impl FnOnce(PathBuf)) {
    let temp_dir = TempDir::with_prefix(prefix).unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    f(temp_dir_path);
}

/// Runs the given async closure with a temporary directory.
pub async fn run_with_tempdir<F, Fut>(prefix: &str, f: F)
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: Future<Output = ()>,
{
    let temp_dir = TempDir::with_prefix(prefix).unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    f(temp_dir_path).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[cfg(unix)]
    use crate::{Anvil, Geth, NodeError, Reth};
    #[cfg(unix)]
    use std::{io::Read, os::unix::fs::PermissionsExt};

    /// Writes an executable shell script at `dir/name`.
    #[cfg(unix)]
    fn script(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let program = dir.join(name);
        std::fs::write(&program, format!("#!/bin/sh\n{body}")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    #[cfg(unix)]
    #[test]
    fn kept_geth_stderr_keeps_lines_read_ahead_during_startup() {
        let dir = tempfile::tempdir().unwrap();
        // Each `printf` is one write, so startup's first read gets the line after readiness too.
        let geth = script(
            dir.path(),
            "geth",
            "printf 'HTTP server started endpoint=127.0.0.1:8545 auth=false\\nAdding p2p peer id=0123456789abcdef\\n' >&2\nsleep 5\n",
        );
        let mut geth = Geth::at(&geth).keep_stderr().try_spawn().unwrap();
        geth.wait_to_add_peer("0123456789abcdef00").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn wait_to_add_peer_keeps_lines_read_ahead() {
        let dir = tempfile::tempdir().unwrap();
        let geth = script(
            dir.path(),
            "geth",
            "echo 'HTTP server started endpoint=127.0.0.1:8545 auth=false' >&2\nsleep 1\nprintf 'Adding p2p peer id=aaaaaaaaaaaaaaaa\\nAdding p2p peer id=bbbbbbbbbbbbbbbb\\n' >&2\nsleep 5\n",
        );
        let mut geth = Geth::at(&geth).keep_stderr().try_spawn().unwrap();
        geth.wait_to_add_peer("aaaaaaaaaaaaaaaa00").unwrap();
        geth.wait_to_add_peer("bbbbbbbbbbbbbbbb00").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn kept_anvil_stdout_keeps_lines_read_ahead_during_startup() {
        let dir = tempfile::tempdir().unwrap();
        let anvil = script(
            dir.path(),
            "anvil",
            "printf 'Listening on 127.0.0.1:8545\\nafter readiness\\n'\n",
        );
        let mut anvil = Anvil::at(&anvil).keep_stdout().try_spawn().unwrap();
        let mut rest = String::new();
        anvil.child_mut().stdout.take().unwrap().read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "after readiness\n");
    }

    #[cfg(unix)]
    #[test]
    fn kept_reth_stdout_keeps_lines_read_ahead_during_startup() {
        let dir = tempfile::tempdir().unwrap();
        let reth = script(
            dir.path(),
            "reth",
            "printf 'RPC HTTP server started url=127.0.0.1:8545\\nRPC WS server started url=127.0.0.1:8546\\nRPC auth server started url=127.0.0.1:8551\\nafter readiness\\n'\n",
        );
        let mut reth = Reth::at(&reth).disable_discovery().keep_stdout().try_spawn().unwrap();
        let mut rest = String::new();
        reth.stdout().unwrap().read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "after readiness\n");
    }

    #[test]
    fn test_extract_value_with_equals() {
        let line = "key=value some other text";
        assert_eq!(extract_value("key", line), Some("value"));
    }

    #[test]
    fn test_extract_value_with_colon() {
        let line = "key: value, more text here";
        assert_eq!(extract_value("key", line), Some("value"));
    }

    #[test]
    fn test_extract_value_not_found() {
        let line = "unrelated text";
        assert_eq!(extract_value("key", line), None);
    }

    #[test]
    fn test_extract_value_equals_no_space() {
        let line = "INFO key=";
        assert_eq!(extract_value("key", line), Some(""))
    }

    #[test]
    fn test_extract_value_colon_no_comma() {
        let line = "INFO key: value";
        assert_eq!(extract_value("key", line), Some("value"))
    }

    #[test]
    fn test_extract_http_address() {
        let line = "INFO [07-01|13:20:42.774] HTTP server started                      endpoint=127.0.0.1:8545 auth=false prefix= cors= vhosts=localhost";
        assert_eq!(
            extract_endpoint("endpoint=", line),
            Some(SocketAddr::from(([127, 0, 0, 1], 8545)))
        );
    }

    #[test]
    fn test_extract_udp_address() {
        let line = "Updated local ENR enr=Enr { id: Some(\"v4\"), seq: 2, NodeId: 0x04dad428038b4db230fc5298646e137564fc6861662f32bdbf220f31299bdde7, signature: \"416520d69bfd701d95f4b77778970a5c18fa86e4dd4dc0746e80779d986c68605f491c01ef39cd3739fdefc1e3558995ad2f5d325f9e1db795896799e8ee94a3\", IpV4 UDP Socket: Some(0.0.0.0:30303), IpV6 UDP Socket: None, IpV4 TCP Socket: Some(0.0.0.0:30303), IpV6 TCP Socket: None, Other Pairs: [(\"eth\", \"c984fc64ec0483118c30\"), (\"secp256k1\", \"a103aa181e8fd5df651716430f1d4b504b54d353b880256f56aa727beadd1b7a9766\")], .. }";
        assert_eq!(
            extract_endpoint("IpV4 TCP Socket: ", line),
            Some(SocketAddr::from(([0, 0, 0, 0], 30303)))
        );
    }

    #[cfg(unix)]
    #[test]
    fn graceful_shutdown_reaps_after_force_kill() {
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg("trap '' TERM; while :; do :; done")
            .spawn()
            .unwrap();

        GracefulShutdown::shutdown(&mut child, 0, "sh");

        assert!(child.try_wait().unwrap().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn startup_errors_reap_children() {
        for (name, output, private_key_error) in [
            ("anvil", "\\377\\n", false),
            ("geth", "\\377\\n", false),
            ("reth", "\\377\\n", false),
            ("anvil", "Private Keys\\n(0) 0xinvalid\\n", true),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let program = dir.path().join(name);
            let redirect = if name == "geth" { ">&2" } else { "" };
            std::fs::write(
                &program,
                format!(
                    "#!/bin/sh\necho $$ > \"$0.pid\"\nprintf '{output}' {redirect}\nwhile :; do :; done\n"
                ),
            )
            .unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();

            let result = match name {
                "anvil" => Anvil::at(&program).try_spawn().map(drop),
                "geth" => Geth::at(&program).try_spawn().map(drop),
                "reth" => Reth::at(&program).try_spawn().map(drop),
                _ => unreachable!(),
            };
            let pid = std::fs::read_to_string(program.with_extension("pid"))
                .unwrap()
                .trim()
                .parse::<libc::pid_t>()
                .unwrap();
            // SAFETY: This only queries the child PID written by our test process.
            let status = unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) };
            let error = std::io::Error::last_os_error();
            if status == 0 {
                // SAFETY: A zero waitpid result confirms this is our live child. Clean it up even
                // when the assertion below will fail.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                    libc::waitpid(pid, std::ptr::null_mut(), 0);
                }
            }

            if private_key_error {
                assert!(matches!(result, Err(NodeError::FromHexError(_))), "{result:?}");
            } else {
                assert!(matches!(result, Err(NodeError::ReadLineError(_))), "{name}: {result:?}");
            }
            assert_eq!(status, -1, "{name} child was not reaped");
            assert_eq!(error.raw_os_error(), Some(libc::ECHILD), "{name}: {error}");
        }
    }
}
