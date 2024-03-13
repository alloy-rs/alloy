//! Message Sign Example

use alloy_signer::{LocalWallet, Signer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup up wallet.
    let wallet = LocalWallet::random();

    // Optionally, the wallet's chain id can be set, in order to use EIP-155
    // replay protection with different chains
    let wallet = wallet.with_chain_id(Some(1337));

    // The message to sign.
    let message = b"hello";

    // Sign the message asynchronously with the wallet.
    let signature = wallet.sign_message(message).await?;

    println!("Signature produced by {:?}: {:?}", wallet.address(), signature);
    println!(
        "Signature recovered address: {:?}",
        signature.recover_address_from_msg(&message[..]).unwrap()
    );

    Ok(())
}
