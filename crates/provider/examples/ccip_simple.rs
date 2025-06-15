//! Example demonstrating simple CCIP (EIP-3668) usage.
//!
//! This example requires the "reqwest" feature to be enabled.

#[cfg(feature = "reqwest")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use alloy_primitives::{address, bytes};
    use alloy_provider::{Provider, ProviderBuilder};
    use alloy_rpc_types_eth::TransactionRequest;

    // Create a provider
    let provider = ProviderBuilder::new().connect_http("https://eth.llamarpc.com".parse()?);

    // Create a transaction that might trigger OffchainLookup
    let tx = TransactionRequest::default()
        .to(address!("1234567890123456789012345678901234567890"))
        .input(bytes!("deadbeef").into());

    // Make a call with CCIP support
    // The `ccip()` method enables automatic resolution of OffchainLookup errors
    let result = provider
        .call(tx)
        .ccip(provider.clone()) // Pass the provider for callback execution
        .await?;

    println!("Call result: 0x{}", alloy_primitives::hex::encode(result));

    Ok(())
}

#[cfg(not(feature = "reqwest"))]
fn main() {
    eprintln!("This example requires the 'reqwest' feature to be enabled.");
    eprintln!("Run with: cargo run --example ccip_simple --features reqwest");
}

