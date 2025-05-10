# signer-stronghold

An Ethereum [signer](https://docs.rs/alloy-signer/latest/alloy_signer/trait.Signer.html) for a Stronghold.

## What is a Stronghold?

From [the documentation](https://github.com/iotaledger/stronghold.rs/blob/iota-stronghold-v2.1.0/client/src/types/stronghold.rs#L77-L82):

> The Stronghold is a secure storage for sensitive data. Secrets that are stored inside
> a Stronghold can never be read, but only be accessed via cryptographic procedures. Data inside
> a Stronghold is heavily protected by the `Runtime` by either being encrypted at rest, having
> kernel supplied memory guards, that prevent memory dumps, or a combination of both. The Stronghold
> also persists data written into a Stronghold by creating Snapshots of the current state. The
> Snapshot itself is encrypted and can be accessed by a key.

[Learn more here](https://github.com/iotaledger/stronghold.rs/tree/iota-stronghold-v2.1.0)


## Usage

```bash
## treat this environment variable with the same care as a private key

export PASSPHRASE=$(openssl rand -hex 48) # or whatever you want
```

### Basic Usage

```rust
use signer_stronghold::StrongholdSigner;

let chain_id = Some(1);
let signer = StrongholdSigner::new(chain_id).unwrap();

let message = vec![0, 1, 2, 3];

let sig = signer.sign_message(&message).await.unwrap();
assert_eq!(sig.recover_address_from_msg(message).unwrap(), signer.address());
```

### With Custom Path

```rust
use signer_stronghold::StrongholdSigner;
use std::path::PathBuf;

let chain_id = Some(1);
let custom_path = PathBuf::from("/path/to/my_custom.stronghold");
let signer = StrongholdSigner::new_from_path(custom_path, chain_id).unwrap();

// Use signer just like the default one
let message = vec![0, 1, 2, 3];
let sig = signer.sign_message(&message).await.unwrap();
```

## License

This project is licensed under the MIT License - see the [LICENSE-MIT](LICENSE-MIT) file for details.
