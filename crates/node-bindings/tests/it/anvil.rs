use alloy_node_bindings::Anvil;

#[test]
fn can_launch_anvil() {
    if !ci_info::is_ci() {
        return;
    }

    let _ = Anvil::new().spawn();
}

#[test]
fn can_launch_anvil_with_more_accounts() {
    if !ci_info::is_ci() {
        return;
    }

    let _ = Anvil::new().arg("--accounts").arg("20").spawn();
}

#[test]
fn assert_chain_id() {
    if !ci_info::is_ci() {
        return;
    }

    let id = 99999;
    let anvil = Anvil::new().chain_id(id).spawn();
    assert_eq!(anvil.chain_id(), id);
}

#[test]
fn assert_chain_id_without_rpc() {
    if !ci_info::is_ci() {
        return;
    }

    let anvil = Anvil::new().spawn();
    assert_eq!(anvil.chain_id(), 31337);
}
