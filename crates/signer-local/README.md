# alloy-signer-local

Local signer implementations:

- [K256 private key](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/type.PrivateKeySigner.html)
- [Secp256k1 private key](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/type.Secp256k1Signer.html) (feature-gated)
- [Mnemonic phrase](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/struct.MnemonicBuilder.html)
- [YubiHSM2](https://docs.rs/alloy-signer-local/latest/alloy_signer_local/type.YubiSigner.html)

## Features

- `keystore`: enables Ethereum keystore functionality on the `PrivateKeySigner` and `Secp256k1Signer` types.
- `mnemonic`: enables BIP-39 mnemonic functionality for building `PrivateKeySigner`s.
- `secp256k1`: enables the `Secp256k1Signer` type, an alternative signer implementation using the [`secp256k1`] crate instead of [`k256`].
- `yubihsm`: enables `LocalSigner`s with [YubiHSM2] support.

## Secp256k1 vs K256

This crate provides two ECDSA implementations:

- **`PrivateKeySigner`** (default): Uses the [`k256`] crate, a pure Rust implementation.
- **`Secp256k1Signer`** (feature-gated): Uses the [`secp256k1`] crate, Rust bindings to [libsecp256k1].

Both implementations produce identical signatures and addresses for the same private key. The `secp256k1` crate may offer better performance in some scenarios due to its optimized C implementation.

[`k256`]: https://docs.rs/k256
[`secp256k1`]: https://docs.rs/secp256k1
[libsecp256k1]: https://github.com/bitcoin-core/secp256k1
[YubiHSM2]: https://www.yubico.com/products/hardware-security-module/
