# alloy-signers

Ethereum signer abstraction.

You can implement the `Signer` trait to extend functionality to other signers
such as Hardware Security Modules, KMS etc.

Supported signers:
- [Private key](./src/wallet)
- [Ledger](./src/ledger)
- [Trezor](./src/trezor)
- [YubiHSM2](./src/wallet/yubi.rs)
- [AWS KMS](./src/aws)

## Examples

<!-- TODO
```rust,no_run
# use ethers_signers::{LocalWallet, Signer};
# use ethers_core::{k256::ecdsa::SigningKey, types::TransactionRequest};
# async fn foo() -> Result<(), Box<dyn std::error::Error>> {
// instantiate the wallet
let wallet = "dcf2cbdd171a21c480aa7f53d77f31bb102282b3ff099c78e3118b37348c72f7"
    .parse::<LocalWallet>()?;

// create a transaction
let tx = TransactionRequest::new()
    .to("vitalik.eth") // this will use ENS
    .value(10000).into();

// sign it
let signature = wallet.sign_transaction(&tx).await?;

// can also sign a message
let signature = wallet.sign_message("hello world").await?;
signature.verify("hello world", wallet.address()).unwrap();
# Ok(())
# }
```
-->

Sign an Ethereum prefixed message ([EIP-712](https://eips.ethereum.org/EIPS/eip-712)):

```rust
use alloy_signer::{LocalWallet, Signer, SignerSync};

let message = "Some data";
let wallet = LocalWallet::random();

// Sign the message
let signature = wallet.sign_message_sync(message.as_bytes())?;

// Recover the signer from the message
let recovered = signature.recover_address_from_msg(message)?;

assert_eq!(recovered, wallet.address());
# Ok::<_, Box<dyn std::error::Error>>(())
```
