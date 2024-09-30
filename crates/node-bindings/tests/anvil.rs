use alloy_node_bindings::Anvil;

/// Run the given function only if we are in a CI environment.
fn ci_only<F>(f: F)
where
    F: FnOnce(),
{
    if ci_info::is_ci() {
        f();
    }
}

#[test]
fn can_launch_anvil() {
    ci_only(|| {
        let _ = Anvil::new().spawn();
    });
}

#[test]
fn can_launch_anvil_with_more_accounts() {
    ci_only(|| {
        let _ = Anvil::new().arg("--accounts").arg("20").spawn();
    })
}

#[test]
fn assert_chain_id() {
    ci_only(|| {
        let id = 99999;
        let anvil = Anvil::new().chain_id(id).spawn();
        assert_eq!(anvil.chain_id(), id);
    })
}

#[test]
fn assert_chain_id_without_rpc() {
    ci_only(|| {
        let anvil = Anvil::new().spawn();
        assert_eq!(anvil.chain_id(), 31337);
    });
}
