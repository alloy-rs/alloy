# alloy-signer-gcp

Ethereum [GCP KMS] signer.

The key must have purpose `ASYMMETRIC_SIGN` and algorithm
`EC_SIGN_SECP256K1_SHA256`; GCP supports secp256k1 only with HSM protection.
The client identity needs `cloudkms.cryptoKeyVersions.viewPublicKey` to
construct a signer and `cloudkms.cryptoKeyVersions.useToSign` to sign.

The example client uses Google Application Default Credentials. Enable this
crate's `eip712` feature to sign EIP-712 typed data.

[GCP KMS]: https://cloud.google.com/kms/docs/
