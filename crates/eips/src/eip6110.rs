//! Contains Deposit request constants, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110): Supply validator deposits on chain

use alloy_primitives::{address, Address};

/// Mainnet deposit contract address.
pub const MAINNET_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("00000000219ab540356cbb839cbe05303d7705fa");
