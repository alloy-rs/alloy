# alloy-signer-tempo

Read-only [Tempo wallet](https://github.com/tempoxyz/wallet) keystore reader
for alloy. Parses the file that `tempo wallet login` writes and exposes the
materialized signer plus optional Keychain-mode metadata. No network I/O.

## On-disk path

`$TEMPO_HOME/wallet/keys.toml`, falling back to `~/.tempo/wallet/keys.toml`
(matches the Tempo CLI). The CLI normally creates this file with Unix mode
`0600`; this crate reads the file without inspecting or enforcing its permissions.

If the file is valid TOML but individual `[[keys]]` entries do not match the
expected schema, those entries are skipped and a warning is emitted through
`tracing`.

## Example

```rust,no_run
use alloy_primitives::address;
use alloy_signer_tempo::{TempoKeystore, TempoLookup};

let store = TempoKeystore::load()?;
match store.find_by_from(address!("0x70997970c51812dc3a010c7d01b50e0d17dc79c8"))? {
    TempoLookup::Direct(signer) => {
        // EOA: the on-disk key IS the wallet account.
        let _ = signer;
    }
    TempoLookup::Keychain(signer, access_key) => {
        // Smart wallet: ephemeral access key signs on behalf of the root wallet.
        let _ = (signer, access_key);
    }
    _ => unreachable!("TempoLookup is `#[non_exhaustive]`"),
}
# Ok::<_, alloy_signer_tempo::TempoSignerError>(())
```

## Scope

In: read `keys.toml`, materialize `PrivateKeySigner`, expose Keychain
metadata, foundry-compatible env vars (`TEMPO_PRIVATE_KEY`,
`TEMPO_ACCESS_KEY`, `TEMPO_ROOT_ACCOUNT`).

Out: writing/rotating keys, decoding `key_authorization` to a typed value
(the RLP bytes are exposed opaquely), any network I/O.

Lookup is metadata extraction, not authorization enforcement. It filters for
available secp256k1 key material and checks expiry at lookup time, but it does
not enforce the recorded chain ID, spending limits, authorization, or expiry
when the signer is later used. Returned signers have no chain ID configured;
configure transaction chain IDs separately.
