# alloy-ens

Ethereum Name Service utilities like namehash, forward & reverse lookups.

Names passed to this crate must already be normalized according to ENSIP-15. The crate does not
perform complete normalization or validation before hashing, DNS-encoding, or resolving input;
`namehash` only applies a legacy rewrite that removes `U+FE0F`.
