# alloy-ens

Ethereum Name Service utilities like namehash, forward & reverse lookups.

Names passed to this crate must already be normalized according to
[ENSIP-15](https://docs.ens.domains/ensip/15/). The crate does not perform complete normalization or
validation before hashing, DNS-encoding, or resolving input; `namehash` only applies a legacy
rewrite that removes `U+FE0F`.

The live DNS-resolution test depends on Ethereum mainnet, a third-party domain, and the ENS
DNSSEC gateway and on-chain trust anchors. It is ignored by default so external DNSSEC outages
do not fail the regular test suite. Run it explicitly with:

```sh
cargo test -p alloy-ens --all-features provider_tests::test_resolve_dns_name -- --ignored --exact
```

Regular tests cover the DNS batch-gateway request and resolver callback using mocked RPC and
gateway responses; they do not validate DNSSEC proofs.
