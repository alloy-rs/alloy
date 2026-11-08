# alloy-signer-turnkey

Ethereum [Turnkey] signer.

The P-256 API key authenticates and stamps Turnkey requests; it is not the
secp256k1 key that signs Ethereum payloads. The Ethereum key remains in
Turnkey and is selected by its address. The API key must be authorized to sign
with that address in the supplied organization.

Enable this crate's `eip712` feature to sign EIP-712 typed data.

[Turnkey]: https://docs.turnkey.com/getting-started/quickstart
