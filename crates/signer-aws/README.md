# alloy-signer-aws

Ethereum [AWS KMS] signer.

The KMS key must be an asymmetric `ECC_SECG_P256K1` key with `SIGN_VERIFY`
usage. Constructing a signer requires `kms:GetPublicKey`; signing requires
`kms:Sign`.

This crate disables the default features of `aws-config` and `aws-sdk-kms`.
Applications that construct the standard AWS client must enable a runtime and
HTTPS client (for example, `rt-tokio` and `default-https-client`) in their own
AWS SDK dependencies, or supply an otherwise configured client.

Enable this crate's `eip712` feature to sign EIP-712 typed data.

[AWS KMS]: https://aws.amazon.com/kms
