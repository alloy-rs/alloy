use alloy_node_bindings::{utils::run_with_tempdir_sync, Geth};
use k256::ecdsa::SigningKey;

#[test]
fn port_0() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("geth-test-", |_| {
        let _geth = Geth::new().disable_discovery().port(0u16).spawn();
    });
}

#[test]
fn p2p_port() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("geth-test-", |temp_dir_path| {
        let geth = Geth::new().disable_discovery().data_dir(temp_dir_path).spawn();
        let p2p_port = geth.p2p_port();
        assert!(p2p_port.is_some());
    });
}

#[test]
fn explicit_p2p_port() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("geth-test-", |temp_dir_path| {
        // if a p2p port is explicitly set, it should be used
        let geth = Geth::new().p2p_port(1234).data_dir(temp_dir_path).spawn();
        let p2p_port = geth.p2p_port();
        assert_eq!(p2p_port, Some(1234));
    });
}

#[test]
fn dev_mode() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("geth-test-", |temp_dir_path| {
        // dev mode should not have a p2p port, and dev should be the default
        let geth = Geth::new().data_dir(temp_dir_path).spawn();
        let p2p_port = geth.p2p_port();
        assert!(p2p_port.is_none(), "{p2p_port:?}");
    })
}

#[test]
#[ignore = "fails on geth >=1.14"]
#[allow(deprecated)]
fn clique_correctly_configured() {
    if !ci_info::is_ci() {
        return;
    }

    run_with_tempdir_sync("geth-test-", |temp_dir_path| {
        let private_key = SigningKey::random(&mut rand::thread_rng());
        let geth = Geth::new()
            .set_clique_private_key(private_key)
            .chain_id(1337u64)
            .data_dir(temp_dir_path)
            .spawn();

        assert!(geth.p2p_port().is_some());
        assert!(geth.clique_private_key().is_some());
        assert!(geth.genesis().is_some());
    })
}
