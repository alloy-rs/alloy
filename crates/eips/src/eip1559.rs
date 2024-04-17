//! [EIP-1559] constants, helpers, and types.
//!
//! [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559

mod basefee;
pub use basefee::BaseFeeParams;

mod constants;
pub use constants::{
    DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR, DEFAULT_ELASTICITY_MULTIPLIER,
    ETHEREUM_BLOCK_GAS_LIMIT, INITIAL_BASE_FEE, MIN_PROTOCOL_BASE_FEE, MIN_PROTOCOL_BASE_FEE_U256,
};

mod helpers;
pub use helpers::calc_next_block_base_fee;
