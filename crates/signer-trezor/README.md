# alloy-signer-trezor

Asynchronous Ethereum signer for [Trezor] devices.

## Device setup

Exactly one Trezor device or emulator must be discoverable. The signer reconnects to that unique
device for each request, so device operations should not be run concurrently. Unlock the device and
complete signing prompts on it. Construction requires firmware 1.11.1 or newer for firmware major
version 1, or 2.5.1 or newer for firmware major version 2.

Although the public signing API is asynchronous, the underlying USB calls are blocking and can
block an executor thread. Address queries do not request on-device display confirmation.

## Supported operations

- EIP-191 personal messages through `Signer::sign_message`.
- Legacy and EIP-1559 transactions, including EIP-1559 access lists, through `TxSigner`.

Raw digest and EIP-712 typed-data signing are not supported. EIP-2930, EIP-4844, EIP-7702, and
other transaction types return an unsupported-transaction error. Passing a 32-byte digest to
`sign_message` signs those bytes as an EIP-191 message; it does not sign the digest directly.

## Chain IDs

The optional chain ID passed to `TrezorSigner::new` applies only to transaction signing. `Some(id)`
fills a transaction that has no chain ID and rejects a different transaction chain ID before the
device is prompted. `None` leaves the transaction unchanged. It does not affect message signatures.

## Example

```no_run
use alloy_signer::Signer;
use alloy_signer_trezor::{HDPath, TrezorSigner};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let signer = TrezorSigner::new(HDPath::TrezorLive(0), Some(1)).await?;
let message = b"hello";
let signature = signer.sign_message(message).await?;

assert_eq!(signature.recover_address_from_msg(message)?, signer.address());
# Ok(())
# }
```

[Trezor]: https://trezor.io
