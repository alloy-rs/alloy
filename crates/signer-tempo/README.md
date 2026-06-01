# alloy-signer-tempo

Read-only [Tempo wallet](https://github.com/tempoxyz/wallet) keystore reader
for alloy. Parses the file that `tempo wallet login` writes and exposes the
materialized signer plus optional Keychain-mode metadata. No network I/O.

## On-disk path

`$TEMPO_HOME/wallet/keys.toml`, falling back to `~/.tempo/wallet/keys.toml`
(Unix mode `0600`). Matches the Tempo CLI exactly.

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
