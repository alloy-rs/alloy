# Examples

These examples demonstrate the main features of Alloy and how to use them. 
To run an example, use the command `cargo run --example <Example>`.

```sh
cargo run --example mnemonic_signer
```

---

## Table of Contents

- [ ] Address book
- [ ] Anvil
    - [ ] Boot anvil
    - [ ] Deploy contracts
    - [ ] Fork
    - [ ] Testing
- [ ] Big numbers
    - [ ] Comparison and equivalence
    - [ ] Conversion
    - [ ] Creating Instances
    - [ ] Math operations
    - [ ] Utilities
- [x] Contracts
    - [x] [Deploy from artifact](./contracts/examples/deploy_from_artifact.rs)
    - [x] [Deploy from contract](./contracts/examples/deploy_from_contract.rs)
    - [x] [Generate](./contracts/examples/generate.rs)
- [ ] Events
  - [ ] Logs and filtering
  - [ ] Solidity topics
- [ ] Middleware
  - [ ] Builder
  - [ ] Create custom middleware
  - [ ] Gas escalator
  - [ ] Gas oracle
  - [ ] Nonce manager
  - [ ] Policy
  - [ ] Signer
  - [ ] Time lag
  - [ ] Transformer
- [ ] Providers
  - [ ] Http
  - [ ] IPC
  - [ ] Mock 
  - [ ] Quorum
  - [ ] Retry
  - [ ] RW
  - [ ] WS
- [ ] Queries
  - [ ] Blocks
  - [ ] Contracts
  - [ ] Events
  - [ ] Paginated logs
  - [ ] UniswapV2 pair
  - [ ] Transactions
- [ ] Subscriptions
  - [ ] Watch blocks
  - [ ] Subscribe events by type
  - [ ] Subscribe logs
- [ ] Transactions
  - [ ] Call override
  - [ ] Create raw transaction
  - [ ] Create typed transaction
  - [ ] Decode input
  - [ ] EIP-1559
  - [ ] ENS
  - [ ] Estimate gas
  - [ ] Get gas price
  - [ ] Get gas price USD
  - [ ] Remove liquidity
  - [ ] Set gas for a transaction
  - [ ] Send raw transaction
  - [ ] Send typed transaction
  - [ ] Trace
  - [ ] Transaction receipt
  - [ ] Transaction status
  - [ ] Transfer ETH
  - [ ] Transfer ERC20 token
- [ ] Wallets
  - [ ] AWS signer
  - [ ] GCP signer
  - [x] [Ledger signer](./wallets/examples/ledger_signer.rs)
  - [x] [Private key signer](./wallets/examples/private_key_signer.rs)
  - [x] [Mnemonic signer](./wallets/examples/mnemonic_signer.rs)
  - [x] [Sign message](./wallets/examples/sign_message.rs)
  - [x] [Sign permit hash](./wallets/examples/sign_permit_hash.rs)
  - [x] [Trezor signer](./wallets/examples/trezor_signer.rs)
  - [x] [Yubi signer](./wallets/examples/yubi_signer.rs)
  - [ ] Keystore signer