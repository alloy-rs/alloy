# alloy-signer-local

Local wallet implementations:
- [K256 private key](./src/private_key.rs)
- [YubiHSM2](./src/yubi.rs)

## Features

- `keystore`: enables Ethereum keystore functionality on the `LocalSigner` type.
- `mnemonic`: enables BIP-39 mnemonic functionality for building `LocalSigner`s.
- `yubihsm`: enables `Wallet`s with [YubiHSM2] support.

[YubiHSM2]: https://www.yubico.com/products/hardware-security-module/
