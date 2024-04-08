# alloy-signer-wallet

Local wallet implementations:
- [K256 private key](./src/private_key.rs)
- [YubiHSM2](./src/yubi.rs)

## Features

- `keystore`: enables Ethereum keystore functionality on the `LocalWallet` type.
- `mnemonic`: enables BIP-39 mnemonic functionality for building `LocalWallet`s.
- `yubihsm`: enables `Wallet`s with [YubiHSM2] support.

[YubiHSM2]: https://www.yubico.com/products/hardware-security-module/
