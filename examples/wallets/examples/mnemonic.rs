//! # Mnemonic Wallet Example

use alloy_signer::{coins_bip39::English, MnemonicBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let phrase = "work man father plunge mystery proud hollow address reunion sauce theory bonus";
    let index = 0u32;
    let password = "TREZOR123";

    // Access mnemonic phrase with password
    // Child key at derivation path: m/44'/60'/0'/0/{index}
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(phrase)
        .index(index)?
        // Use this if your mnemonic is encrypted
        .password(password)
        .build()?;

    eprintln!("Wallet: {wallet:?}");

    Ok(())
}
