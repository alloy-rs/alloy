# alloy-signer-local

Local signer implementations:

- [K256 private key](./src/private_key.rs)
- [Mnemonic phrase](./src/mnemonic.rs)
- [YubiHSM2](./src/yubi.rs)

## Features

- `keystore`: enables Ethereum keystore functionality on the `PrivateKeySigner` type.
- `mnemonic`: enables BIP-39 mnemonic functionality for building `PrivateKeySigner`s.
- `yubihsm`: enables `LocalSigner`s with [YubiHSM2] support.

[YubiHSM2]: https://www.yubico.com/products/hardware-security-module/
