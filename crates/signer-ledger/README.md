# alloy-signer-ledger

Asynchronous Ethereum signer for [Ledger] devices.

## Device setup

Unlock the device and open its Ethereum app before constructing a signer. On native targets,
`LedgerSigner::new` connects to the first compatible device and keeps an exclusive transport open;
share that transport through `LedgerSigner::new_with_transport` when using multiple derivation
paths. Signing requests wait for approval on the device. Address queries do not request on-device
confirmation.

## Supported operations

- Ethereum transactions through `TxSigner`, including legacy, EIP-2930, and EIP-1559 signing
  payloads.
- EIP-191 personal messages through `Signer::sign_message`.
- EIP-712 typed data with the `eip712` feature and Ledger Ethereum app version 1.6.0 or newer. The
  device receives the domain separator and struct hash.
- EIP-7702 authorizations with the `eip7702` feature.

Raw digest signing through `Signer::sign_hash` is not supported. Passing a 32-byte digest to
`sign_message` signs those bytes as an EIP-191 message; it does not sign the digest directly.

## Chain IDs

Set each transaction's chain ID explicitly before signing. The optional chain ID passed to
`LedgerSigner::new` applies only to transaction signing: `Some(id)` rejects a transaction carrying
a different ID, but must not be relied on to supply a missing ID. `None` leaves the transaction
unchanged. The signer setting does not constrain messages, EIP-712 domains, or EIP-7702
authorizations.

## Example

```no_run
use alloy_signer::Signer;
use alloy_signer_ledger::{HDPath, LedgerSigner};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let signer = LedgerSigner::new(HDPath::LedgerLive(0), Some(1)).await?;
let message = b"hello";
let signature = signer.sign_message(message).await?;

assert_eq!(signature.recover_address_from_msg(message)?, signer.address());
# Ok(())
# }
```

[Ledger]: https://www.ledger.com
