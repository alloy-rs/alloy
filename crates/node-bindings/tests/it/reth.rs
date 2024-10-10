// These tests should use a different datadir for each `reth` instance spawned.

use alloy_genesis::Genesis;
use alloy_node_bindings::{utils::run_with_tempdir_sync, Reth, RethInstance};

/// The default HTTP port for Reth.
const DEFAULT_HTTP_PORT: u16 = 8545;

/// The default WS port for Reth.
const DEFAULT_WS_PORT: u16 = 8546;

/// The default auth port for Reth.
const DEFAULT_AUTH_PORT: u16 = 8551;

/// The default P2P port for Reth.
const DEFAULT_P2P_PORT: u16 = 30303;

#[test]
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
#[cfg_attr(windows, ignore)]
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
