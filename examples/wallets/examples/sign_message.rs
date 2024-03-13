//! Message Sign Example

use alloy_signer::{LocalWallet, Signer, SignerSync};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup up wallet.
    let wallet = LocalWallet::random();

    // Optionally, the wallet's chain id can be set, in order to use EIP-155
    // replay protection with different chains
    let wallet = wallet.with_chain_id(Some(1337));

    // The message to sign.
    let message = b"hello";

    // Sign message synchronously from the wallet and print out signature produced.
    let signature = wallet.sign_message_sync(message)?;

    println!("Signature produced by {:?}: {:?}", wallet.address(), signature);
    println!(
        "Signature recovered address: {:?}",
        signature.recover_address_from_msg(&message[..]).unwrap()
    );

    // Sign message asynchronously from the wallet and print out signature produced.
    let signature = wallet.sign_message(message).await?;

    println!("Signature produced by {:?}: {:?}", wallet.address(), signature);
    println!(
        "Signature recovered address: {:?}",
        signature.recover_address_from_msg(&message[..]).unwrap()
    );

    Ok(())
}
