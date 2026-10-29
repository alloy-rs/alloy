# alloy-signer-ledger

Asynchronous Ethereum signer for [Ledger] devices.

## Device setup

Unlock the device and open its Ethereum app before constructing a signer. `LedgerSigner::new`
connects to the first compatible USB HID device and keeps an exclusive transport open; share that
transport through `LedgerSigner::new_with_transport` when using multiple derivation paths. The
signer is generic over the `coins_ledger` transport, so `new_with_transport` also accepts any other
`LedgerTransport` implementation, such as an emulator. Signing requests wait for approval on the
device. Address queries do not request on-device confirmation.

## Supported operations

- Ethereum transactions through `TxSigner`, including legacy, EIP-2930, and EIP-1559 signing
  payloads.
- EIP-191 personal messages through `Signer::sign_message`.
- EIP-712 typed data with the `eip712` feature and Ledger Ethereum app version 1.6.0 or newer. The
  device receives the domain separator and struct hash.
- EIP-7702 authorizations with the `eip7702` feature and Ledger Ethereum app version 1.17.0 or
  newer.

Raw digest signing through `Signer::sign_hash` is not supported. Passing a 32-byte digest to
`sign_message` signs those bytes as an EIP-191 message; it does not sign the digest directly.

## Chain IDs

The optional chain ID passed to `LedgerSigner::new` applies only to transaction signing: `Some(id)`
fills a transaction that has no chain ID and rejects a different transaction chain ID before the
device is prompted. `None` leaves the transaction unchanged. The signer setting does not constrain
messages, EIP-712 domains, or EIP-7702 authorizations.

## Testing against Speculos

The `speculos` feature enables `coins_ledger`'s transport for Ledger's [Speculos] emulator, and the
crate's `tests/speculos.rs` drives the signer through the Ethereum app in it: address derivation,
message, transaction and EIP-7702 signing with on-screen approval, rejection, and the APDU chunking
regression from [app-ethereum#409]. The tests skip unless `SPECULOS_APDU_ADDR` is set.

Ledger publishes the app for each model on the [app-ethereum] releases page; `nanos2` is the Nano S
Plus. Start Speculos from the directory holding the app, with the seed the tests expect:

```sh
gh release download 1.23.0 -R LedgerHQ/app-ethereum -p app-1.23.0-nanos2.elf
docker run -d --rm --name speculos -v "$PWD":/apps -p 9999:9999 -p 5000:5000 \
  ghcr.io/ledgerhq/speculos:v0.27.1 \
  --model nanosp --display headless --apdu-port 9999 --api-port 5000 \
  --seed "test test test test test test test test test test test junk" \
  /apps/app-1.23.0-nanos2.elf
SPECULOS_APDU_ADDR=127.0.0.1:9999 SPECULOS_API_URL=http://127.0.0.1:5000 SPECULOS_MODEL=nanosp \
  cargo test -p alloy-signer-ledger --all-features --test speculos
docker stop speculos
```

The tests navigate the button-driven Nano models (`nanosp`, `nanox`); Flex and Stax need touch
input and are not supported yet. On macOS, AirPlay Receiver holds port 5000: map the API port
elsewhere, such as `-p 5001:5000`, and point `SPECULOS_API_URL` at it.

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
[Speculos]: https://github.com/LedgerHQ/speculos
[app-ethereum]: https://github.com/LedgerHQ/app-ethereum/releases
[app-ethereum#409]: https://github.com/LedgerHQ/app-ethereum/issues/409
