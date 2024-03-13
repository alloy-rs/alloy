# Examples

These examples demonstrate the main features of Alloy and how to use them. 
To run an example, use the command `cargo run --example <Example>`.

```sh
cargo run --example mnemonic
```

---

## Table of Contents

- [ ] Address book
- [ ] Anvil
    - [x] Boot anvil
    - [ ] Deploy contracts
    - [x] Fork
    - [ ] Testing
- [ ] Big numbers
    - [ ] Comparison and equivalence
    - [ ] Conversion
    - [ ] Creating Instances
    - [ ] Math operations
    - [ ] Utilities
- [ ] Contracts
    - [ ] Abigen
    - [ ] Compile
    - [ ] Creating Instances
    - [ ] Deploy Anvil
    - [ ] Deploy from ABI and bytecode
    - [ ] Deploy Moonbeam
    - [ ] Events
    - [ ] Events with meta
    - [ ] Methods
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
  - [ ] AWS
  - [ ] GCP
  - [x] [Ledger](./wallets/examples/ledger.rs)
  - [x] [Local](./wallets/examples/local.rs)
  - [x] [Mnemonic](./wallets/examples/mnemonic.rs)
  - [x] [Sign message](./wallets/examples/sign_message.rs)
  - [x] [Sign permit hash](./wallets/examples/sign_permit_hash.rs)
  - [x] [Trezor](./wallets/examples/trezor.rs)
  - [x] [Yubi](./wallets/examples/yubi.rs)
  - [ ] Keystore