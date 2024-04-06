use crate::eip1559::constants::{
    DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR, DEFAULT_ELASTICITY_MULTIPLIER,
};

/// BaseFeeParams contains the config parameters that control block base fee computation
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseFeeParams {
    /// The base_fee_max_change_denominator from EIP-1559
    pub max_change_denominator: u128,
    /// The elasticity multiplier from EIP-1559
    pub elasticity_multiplier: u128,
}

impl BaseFeeParams {
    /// Get the base fee parameters for Ethereum mainnet
    pub const fn ethereum() -> BaseFeeParams {
        BaseFeeParams {
            max_change_denominator: DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR as u128,
            elasticity_multiplier: DEFAULT_ELASTICITY_MULTIPLIER as u128,
        }
    }
}
