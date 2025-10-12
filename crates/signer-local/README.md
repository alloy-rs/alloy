# alloy-signer-local

Local signer implementations:

- [K256 private key](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/type.PrivateKeySigner.html)
- [Mnemonic phrase](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/struct.MnemonicBuilder.html)
- [YubiHSM2](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/type.YubiSigner.html)

## Features

- `keystore`: enables Ethereum keystore functionality on the `PrivateKeySigner` type.
- `mnemonic`: enables BIP-39 mnemonic functionality for building `PrivateKeySigner`s.
- `yubihsm`: enables `LocalSigner`s with [YubiHSM2] support.

[YubiHSM2]: https://www.yubico.com/products/hardware-security-module/
