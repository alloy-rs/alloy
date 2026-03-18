// These tests should use a different datadir for each `reth` instance spawned.

use alloy_genesis::Genesis;
use alloy_node_bindings::{utils::run_with_tempdir_sync, Reth, RethInstance};
use std::{fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// The default HTTP port for Reth.
const DEFAULT_HTTP_PORT: u16 = 8545;

/// The default WS port for Reth.
const DEFAULT_WS_PORT: u16 = 8546;

/// The default auth port for Reth.
const DEFAULT_AUTH_PORT: u16 = 8551;

/// The default P2P port for Reth.
const DEFAULT_P2P_PORT: u16 = 30303;

#[test]
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().data_dir(temp_dir_path).spawn();

        assert_ports(&reth, false);
    });
}

#[test]
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_sepolia() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().chain_or_path("sepolia").data_dir(temp_dir_path).spawn();

        assert_ports(&reth, false);
    });
}

#[test]
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_dev() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().dev().disable_discovery().data_dir(temp_dir_path).spawn();

        assert_ports(&reth, true);
    });
}

#[test]
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_dev_custom_genesis() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
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
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_dev_custom_blocktime() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
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
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_p2p_instances() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().instance(100).data_dir(temp_dir_path).spawn();

        assert_ports(&reth, false);

        run_with_tempdir_sync("reth-test-", |temp_dir_path| {
            let reth = Reth::new().instance(101).data_dir(temp_dir_path).spawn();

            assert_ports(&reth, false);
        });
    });
}

// Tests that occupy the same port are combined so they are ran sequentially, to prevent
// flakiness.
#[test]
#[cfg_attr(windows, ignore = "no reth on windows")]
fn can_launch_reth_custom_ports() {
    if !ci_info::is_ci() {
        return;
    }

    // Assert that all ports are default if no custom ports are set
    // and the instance is set to 0.
    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().instance(0).data_dir(temp_dir_path).spawn();

        assert_eq!(reth.http_port(), DEFAULT_HTTP_PORT);
        assert_eq!(reth.ws_port(), DEFAULT_WS_PORT);
        assert_eq!(reth.auth_port(), Some(DEFAULT_AUTH_PORT));
        assert_eq!(reth.p2p_port(), Some(DEFAULT_P2P_PORT));
    });

    // Assert that only the HTTP port is set and the rest are default.
    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().http_port(8577).data_dir(temp_dir_path).spawn();

        assert_eq!(reth.http_port(), 8577);
        assert_eq!(reth.ws_port(), DEFAULT_WS_PORT);
        assert_eq!(reth.auth_port(), Some(DEFAULT_AUTH_PORT));
        assert_eq!(reth.p2p_port(), Some(DEFAULT_P2P_PORT));
    });

    // Assert that all ports can be set.
    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new()
            .http_port(8577)
            .ws_port(8578)
            .auth_port(8579)
            .p2p_port(30307)
            .data_dir(temp_dir_path)
            .spawn();

        assert_eq!(reth.http_port(), 8577);
        assert_eq!(reth.ws_port(), 8578);
        assert_eq!(reth.auth_port(), Some(8579));
        assert_eq!(reth.p2p_port(), Some(30307));
    });

    // Assert that the HTTP port is picked by the OS and the rest are default.
    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let reth = Reth::new().http_port(0).data_dir(temp_dir_path).spawn();

        // Assert that a random unused port is used picked by the OS.
        assert_ne!(reth.http_port(), DEFAULT_HTTP_PORT);

        assert_eq!(reth.ws_port(), DEFAULT_WS_PORT);
        assert_eq!(reth.auth_port(), Some(DEFAULT_AUTH_PORT));
        assert_eq!(reth.p2p_port(), Some(DEFAULT_P2P_PORT));
    });
}

#[test]
#[cfg_attr(windows, ignore = "uses a unix shell script as a fake reth binary")]
fn genesis_runs_reth_init_with_datadir_before_node() {
    run_with_tempdir_sync("reth-test-", |temp_dir_path| {
        let script_path = temp_dir_path.join("fake-reth.sh");
        write_fake_reth(&script_path);

        let data_dir = temp_dir_path.join("datadir");
        let genesis = Genesis::default().with_timestamp(0x123456).with_gas_limit(0xabcdef);

        let reth = Reth::new()
            .path(&script_path)
            .dev()
            .disable_discovery()
            .data_dir(&data_dir)
            .genesis(genesis.clone())
            .spawn();

        assert_eq!(reth.data_dir(), Some(&data_dir));
        assert_eq!(reth.genesis(), Some(&genesis));

        let init_args =
            fs::read_to_string(temp_dir_path.join("init-args.txt")).expect("init args file");
        assert!(init_args.contains("--datadir"));
        assert!(init_args.contains(data_dir.to_string_lossy().as_ref()));
        assert!(init_args.contains("--chain"));

        let node_args =
            fs::read_to_string(temp_dir_path.join("node-args.txt")).expect("node args file");
        assert!(node_args.contains("--datadir"));
        assert!(node_args.contains(data_dir.to_string_lossy().as_ref()));

        let initialized_genesis: Genesis = serde_json::from_slice(
            &fs::read(temp_dir_path.join("init-genesis.json")).expect("captured genesis file"),
        )
        .expect("deserialize captured genesis");
        assert_eq!(initialized_genesis, genesis);
    });
}

// Asserts that the ports are set correctly for the given Reth instance.
fn assert_ports(reth: &RethInstance, dev: bool) {
    // Changes to the following port numbers for each instance:
    // - `HTTP_RPC_PORT`: default - `instance` + 1
    // - `WS_RPC_PORT`: default + `instance` * 2 - 2
    // - `AUTH_PORT`: default + `instance` * 100 - 100
    // - `DISCOVERY_PORT`: default + `instance` - 1
    assert_eq!(reth.http_port(), DEFAULT_HTTP_PORT - reth.instance() + 1);
    assert_eq!(reth.ws_port(), DEFAULT_WS_PORT + reth.instance() * 2 - 2);
    assert_eq!(reth.auth_port(), Some(DEFAULT_AUTH_PORT + reth.instance() * 100 - 100));
    assert_eq!(
        reth.p2p_port(),
        if dev { None } else { Some(DEFAULT_P2P_PORT + reth.instance() - 1) }
    );
}

#[cfg(unix)]
fn write_fake_reth(path: &Path) {
    let script = r#"#!/bin/sh
set -eu

script_dir=$(cd "$(dirname "$0")" && pwd)

if [ "$1" = "init" ]; then
  shift
  printf '%s\n' "$@" > "$script_dir/init-args.txt"
  while [ "$#" -gt 0 ]; do
    if [ "$1" = "--chain" ]; then
      cp "$2" "$script_dir/init-genesis.json"
      break
    fi
    shift
  done
  exit 0
fi

if [ "$1" = "node" ]; then
  shift
  if [ ! -f "$script_dir/init-args.txt" ]; then
    echo "ERROR init was not called before node"
    exit 1
  fi

  printf '%s\n' "$@" > "$script_dir/node-args.txt"
  echo "RPC HTTP server started url=127.0.0.1:8545"
  echo "RPC WS server started url=127.0.0.1:8546"
  echo "RPC auth server started url=127.0.0.1:8551"
  trap 'exit 0' TERM INT
  while true; do
    sleep 1
  done
fi

echo "ERROR unexpected subcommand: $1"
exit 1
"#;

    fs::write(path, script).expect("write fake reth script");
    let mut perms = fs::metadata(path).expect("fake reth metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod fake reth");
}

#[cfg(windows)]
fn write_fake_reth(_: &Path) {
    unreachable!("windows test should be ignored");
}
