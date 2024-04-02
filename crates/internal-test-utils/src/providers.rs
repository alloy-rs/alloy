use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{network::Ethereum, ReqwestProvider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use reqwest::Client;

#[allow(unused, unreachable_pub)]
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

#[allow(unused, unreachable_pub)]
pub fn spawn_anvil() -> (ReqwestProvider<Ethereum>, AnvilInstance) {
    spawn_anvil_with(std::convert::identity)
}

#[allow(unused, unreachable_pub)]
pub fn spawn_anvil_with(
    f: impl FnOnce(Anvil) -> Anvil,
) -> (ReqwestProvider<Ethereum>, AnvilInstance) {
    let anvil = f(Anvil::new()).try_spawn().expect("could not spawn anvil");
    (anvil_http_provider(&anvil), anvil)
}

#[allow(unused, unreachable_pub)]
pub fn anvil_http_provider(anvil: &AnvilInstance) -> ReqwestProvider<Ethereum> {
    http_provider(&anvil.endpoint())
}

#[allow(unused, unreachable_pub)]
pub fn http_provider(url: &str) -> ReqwestProvider<Ethereum> {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    ReqwestProvider::new(RpcClient::new(http, true))
}

#[allow(unused, unreachable_pub)]
pub fn spawn_anvil_fork(rpc_url: &str) -> (ReqwestProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().fork(rpc_url).try_spawn().expect("could not spawn forked anvil");
    (anvil_http_provider(&anvil), anvil)
}
