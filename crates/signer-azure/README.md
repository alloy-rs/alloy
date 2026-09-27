# alloy-signer-azure

Ethereum [Azure Key Vault] signer.

The key must be an elliptic curve key (`EC` or `EC-HSM`) on the `P-256K` curve, in a vault or a
Managed HSM. Constructing a signer requires the `keys/get` permission; signing requires
`keys/sign`. The built-in Key Vault Crypto User role grants both.

This crate uses the Azure SDK's default HTTP client (reqwest with rustls, on tokio) and re-exports
`azure_core`, `azure_identity`, and `azure_security_keyvault_keys` to construct compatible
credentials and clients.

Enable this crate's `eip712` feature to sign EIP-712 typed data.

[Azure Key Vault]: https://learn.microsoft.com/azure/key-vault/
