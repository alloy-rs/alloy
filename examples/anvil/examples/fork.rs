//! Example of spinning up a forked Anvil node.

use alloy_node_bindings::Anvil;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up a forked Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().fork("https://eth.llamarpc.com").spawn();

    println!("Anvil running at `{}`", anvil.endpoint());

    Ok(())
}
