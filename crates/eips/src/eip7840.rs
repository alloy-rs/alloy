//! Contains constants and utility functions for [EIP-7840](https://github.com/ethereum/EIPs/tree/master/EIPS/eip-7840.md)

use crate::{
    eip4844::{self, DATA_GAS_PER_BLOB},
    eip7594, eip7691,
};

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
    /// If not specified, defaults to `max_blob_count` during deserialization.
    pub max_blobs_per_tx: u64,
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
        }
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
    pub const fn next_block_excess_blob_gas(
        &self,
        excess_blob_gas: u64,
        blob_gas_used: u64,
    ) -> u64 {
        (excess_blob_gas + blob_gas_used)
            .saturating_sub(eip4844::DATA_GAS_PER_BLOB * self.target_blob_count)
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
        #[serde(rename = "target")]
        target_blob_count: u64,
        #[serde(rename = "max")]
        max_blob_count: u64,
        #[serde(rename = "baseFeeUpdateFraction")]
        update_fraction: u128,
        min_blob_fee: Option<u128>,
        max_blobs_per_tx: Option<u64>,
    }

    impl From<BlobParams> for SerdeHelper {
        fn from(params: BlobParams) -> Self {
            let BlobParams {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee,
                max_blobs_per_tx,
            } = params;

            Self {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee: (min_blob_fee != eip4844::BLOB_TX_MIN_BLOB_GASPRICE)
                    .then_some(min_blob_fee),
                max_blobs_per_tx: Some(max_blobs_per_tx),
            }
        }
    }

    impl From<SerdeHelper> for BlobParams {
        fn from(helper: SerdeHelper) -> Self {
            let SerdeHelper {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee,
                max_blobs_per_tx,
            } = helper;

            Self {
                target_blob_count,
                max_blob_count,
                update_fraction,
                min_blob_fee: min_blob_fee.unwrap_or(eip4844::BLOB_TX_MIN_BLOB_GASPRICE),
                max_blobs_per_tx: max_blobs_per_tx.unwrap_or(max_blob_count),
            }
        }
    }
}
