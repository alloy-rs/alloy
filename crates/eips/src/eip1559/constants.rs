use alloy_primitives::U256;

/// The default Ethereum block gas limit.
pub const ETHEREUM_BLOCK_GAS_LIMIT: u64 = 30_000_000;

/// The minimum tx fee below which the txpool will reject the transaction.
///
/// Configured to `7` WEI which is the lowest possible value of base fee under mainnet EIP-1559
/// parameters. `BASE_FEE_MAX_CHANGE_DENOMINATOR` <https://eips.ethereum.org/EIPS/eip-1559>
/// is `8`, or 12.5%. Once the base fee has dropped to `7` WEI it cannot decrease further because
/// 12.5% of 7 is less than 1.
///
/// Note that min base fee under different 1559 parameterizations may differ, but there's no
/// significant harm in leaving this setting as is.
pub const MIN_PROTOCOL_BASE_FEE: u64 = 7;

/// Same as [MIN_PROTOCOL_BASE_FEE] but as a U256.
pub const MIN_PROTOCOL_BASE_FEE_U256: U256 = U256::from_limbs([7u64, 0, 0, 0]);

/// Initial base fee as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const INITIAL_BASE_FEE: u64 = 1_000_000_000;

/// Base fee max change denominator as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const DEFAULT_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

/// Elasticity multiplier as defined in [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
pub const DEFAULT_ELASTICITY_MULTIPLIER: u64 = 2;
