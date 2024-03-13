//! Example of spinning up a local Anvil node.

use alloy_node_bindings::Anvil;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spin up a local Anvil node.
    // Ensure `anvil` is available in $PATH
    let anvil = Anvil::new().spawn();

    println!("Anvil running at `{}`", anvil.endpoint());

    Ok(())
}
