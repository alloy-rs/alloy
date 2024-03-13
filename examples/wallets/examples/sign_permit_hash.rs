//! Permit Hash Signing Example

use alloy_primitives::{address, keccak256, U256};
use alloy_signer::{LocalWallet, Signer};
use alloy_sol_types::{eip712_domain, sol, SolStruct};
use serde::Serialize;

sol! {
    #[derive(Debug, Serialize)]
    struct Permit {
        address owner;
        address spender;
        uint256 value;
        uint256 nonce;
        uint256 deadline;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup up wallet.
    let wallet = LocalWallet::random();

    let domain = eip712_domain! {
        name: "Uniswap V2",
        version: "1",
        chain_id: 1,
        verifying_contract: address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"),
        salt: keccak256("test"),
    };

    let permit = Permit {
        owner: wallet.address(),
        spender: address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"),
        value: U256::from(100),
        nonce: U256::from(0),
        deadline: U256::from(0),
    };

    // Derive the EIP-712 signing hash.
    let hash = permit.eip712_signing_hash(&domain);

    // Sign the hash asynchronously with the wallet.
    let signature = wallet.sign_typed_data(&permit, &domain).await?;

    println!(
        "Recovered address matches wallet address: {:?}",
        signature.recover_address_from_prehash(&hash)? == wallet.address()
    );

    println!("Wallet signature matches: {:?}", wallet.sign_hash(&hash).await? == signature);

    Ok(())
}
