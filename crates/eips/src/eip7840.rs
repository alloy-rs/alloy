//! Contains constants and utility functions for [EIP-7840](https://github.com/ethereum/EIPs/tree/master/EIPS/eip-7840.md)

use crate::{
    eip4844::{self, DATA_GAS_PER_BLOB},
    eip7594, eip7691,
};

/// BLOB_BASE_COST represents the minimum execution gas required to include a blob in a block,
/// as defined by [EIP-7918 (Decoupling Blob Gas from Execution Gas)](https://eips.ethereum.org/EIPS/eip-7918).
/// This ensures that even though blob gas and execution gas are decoupled, there is still a base
/// cost in execution gas to include blobs.
pub const BLOB_BASE_COST: u64 = 2_u64.pow(13);

/// Configuration for the blob-related calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(from = "serde_impl::SerdeHelper", into = "serde_impl::SerdeHelper")
)]
pub struct BlobParams {
    /// Target blob count for the block.
    pub target_blob_count: u64,
    /// Max blob count for the block.
    pub max_blob_count: u64,
    /// Update fraction for excess blob gas calculation.
    pub update_fraction: u128,
    /// Minimum gas price for a data blob.
    ///
    /// Not required per EIP-7840 and assumed to be the default
    /// [`eip4844::BLOB_TX_MIN_BLOB_GASPRICE`] if not set.
    pub min_blob_fee: u128,
    /// Maximum number of blobs per transaction.
    ///
    /// Defaults to `max_blob_count` unless set otherwise.
    pub max_blobs_per_tx: u64,
    /// Minimum execution gas required to include a blob in a block.
    ///
    /// Defaults to `0` unless set otherwise.
    pub blob_base_cost: u64,
}

impl BlobParams {
    /// Returns [`BlobParams`] configuration activated with Cancun hardfork.
    pub const fn cancun() -> Self {
        Self {
            target_blob_count: eip4844::TARGET_BLOBS_PER_BLOCK_DENCUN,
            max_blob_count: eip4844::MAX_BLOBS_PER_BLOCK_DENCUN as u64,
            update_fraction: eip4844::BLOB_GASPRICE_UPDATE_FRACTION,
            min_blob_fee: eip4844::BLOB_TX_MIN_BLOB_GASPRICE,
            max_blobs_per_tx: eip4844::MAX_BLOBS_PER_BLOCK_DENCUN as u64,
            blob_base_cost: 0,
        }
    }

    /// Returns [`BlobParams`] configuration activated with Prague hardfork.
    pub const fn prague() -> Self {
        Self {
            target_blob_count: eip7691::TARGET_BLOBS_PER_BLOCK_ELECTRA,
            max_blob_count: eip7691::MAX_BLOBS_PER_BLOCK_ELECTRA,
            update_fraction: eip7691::BLOB_GASPRICE_UPDATE_FRACTION_PECTRA,
            min_blob_fee: eip4844::BLOB_TX_MIN_BLOB_GASPRICE,
            max_blobs_per_tx: eip7691::MAX_BLOBS_PER_BLOCK_ELECTRA,
            blob_base_cost: 0,
        }
    }

    /// Returns [`BlobParams`] configuration activated with Osaka hardfork.
    pub const fn osaka() -> Self {
        Self {
            target_blob_count: eip7691::TARGET_BLOBS_PER_BLOCK_ELECTRA,
            max_blob_count: eip7691::MAX_BLOBS_PER_BLOCK_ELECTRA,
            update_fraction: eip7691::BLOB_GASPRICE_UPDATE_FRACTION_PECTRA,
            min_blob_fee: eip4844::BLOB_TX_MIN_BLOB_GASPRICE,
            max_blobs_per_tx: eip7594::MAX_BLOBS_PER_TX_FUSAKA,
            blob_base_cost: BLOB_BASE_COST,
        }
    }

    /// Set max blobs per transaction on [`BlobParams`].
    pub const fn with_max_blobs_per_tx(mut self, max_blobs_per_tx: u64) -> Self {
        self.max_blobs_per_tx = max_blobs_per_tx;
        self
    }

    /// Set blob base cost on [`BlobParams`].
    pub const fn with_blob_base_cost(mut self, blob_base_cost: u64) -> Self {
        self.blob_base_cost = blob_base_cost;
        self
    }

    /// Returns the maximum available blob gas in a block.
    ///
    /// This is `max blob count * DATA_GAS_PER_BLOB`
    pub const fn max_blob_gas_per_block(&self) -> u64 {
        self.max_blob_count * DATA_GAS_PER_BLOB
    }

    /// Returns the blob gas target per block.
    ///
    /// This is `target blob count * DATA_GAS_PER_BLOB`
    pub const fn target_blob_gas_per_block(&self) -> u64 {
        self.target_blob_count * DATA_GAS_PER_BLOB
    }

    /// Calculates the `excess_blob_gas` value for the next block based on the current block
    /// `excess_blob_gas` and `blob_gas_used`.
    #[inline]
    // #[deprecated(note = "Use `next_block_excess_blob_gas_osaka` instead")]
    pub const fn next_block_excess_blob_gas(
        &self,
        excess_blob_gas: u64,
        blob_gas_used: u64,
    ) -> u64 {
        self.next_block_excess_blob_gas_osaka(excess_blob_gas, blob_gas_used, 0)
    }

    /// Calculates the `excess_blob_gas` value for the next block based on the current block
    /// `excess_blob_gas`, `blob_gas_used` and `base_fee_per_gas`.
    #[inline]
    pub const fn next_block_excess_blob_gas_osaka(
        &self,
        excess_blob_gas: u64,
        blob_gas_used: u64,
        base_fee_per_gas: u64,
    ) -> u64 {
        let next_excess_blob_gas = excess_blob_gas + blob_gas_used;
        let target_blob_gas = self.target_blob_gas_per_block();
        if next_excess_blob_gas < target_blob_gas {
            return 0;
        }

        if self.blob_base_cost as u128 * base_fee_per_gas as u128
            > DATA_GAS_PER_BLOB as u128 * self.calc_blob_fee(excess_blob_gas)
        {
            let scaled_excess = blob_gas_used * (self.max_blob_count - self.target_blob_count)
                / self.max_blob_count;
            excess_blob_gas + scaled_excess
        } else {
            next_excess_blob_gas - target_blob_gas
        }
    }

    /// Calculates the blob fee for block based on its `excess_blob_gas`.
    #[inline]
    pub const fn calc_blob_fee(&self, excess_blob_gas: u64) -> u128 {
        eip4844::fake_exponential(self.min_blob_fee, excess_blob_gas as u128, self.update_fraction)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use crate::{eip4844, eip7840::BlobParams};

    #[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct SerdeHelper {
        #[serde(rename = "baseFeeUpdateFraction")]
        update_fraction: u128,
        #[serde(rename = "max")]
        max_blob_count: u64,
        #[serde(rename = "target")]
        target_blob_count: u64,
        #[serde(skip_serializing)]
        min_blob_fee: Option<u128>,
    }

    impl From<BlobParams> for SerdeHelper {
        fn from(params: BlobParams) -> Self {
            let BlobParams {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee,
                max_blobs_per_tx: _,
                blob_base_cost: _,
            } = params;

            Self {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee: (min_blob_fee != eip4844::BLOB_TX_MIN_BLOB_GASPRICE)
                    .then_some(min_blob_fee),
            }
        }
    }

    impl From<SerdeHelper> for BlobParams {
        fn from(helper: SerdeHelper) -> Self {
            let SerdeHelper { target_blob_count, max_blob_count, update_fraction, min_blob_fee } =
                helper;

            Self {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee: min_blob_fee.unwrap_or(eip4844::BLOB_TX_MIN_BLOB_GASPRICE),
                max_blobs_per_tx: max_blob_count,
                blob_base_cost: 0,
            }
        }
    }
}
