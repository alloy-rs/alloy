//! Contains helpers for dealing with additional parameters of `newPayload` requests.

use crate::{
    BogotaPayloadFields, CancunPayloadFields, MaybeBogotaPayloadFields, MaybeCancunPayloadFields,
    MaybePraguePayloadFields, PraguePayloadFields,
};
use alloc::vec::Vec;
use alloy_consensus::{Block, BlockHeader, Transaction};
use alloy_eips::eip7685::Requests;
use alloy_primitives::{Bytes, B256};

/// Container type for all available additional `newPayload` request parameters that are not present
/// in the `ExecutionPayload` object itself.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadSidecar {
    /// Cancun request params introduced in `engine_newPayloadV3` that are not present in the
    /// `ExecutionPayload`.
    cancun: MaybeCancunPayloadFields,
    /// The EIP-7685 requests provided as additional request params to `engine_newPayloadV4` that
    /// are not present in the `ExecutionPayload`.
    prague: MaybePraguePayloadFields,
    /// EIP-7805 inclusion-list transactions provided as an additional request parameter to
    /// `engine_newPayloadV6` that are not present in the `ExecutionPayload`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "MaybeBogotaPayloadFields::is_none")
    )]
    bogota: MaybeBogotaPayloadFields,
}

impl ExecutionPayloadSidecar {
    /// Extracts the [`ExecutionPayloadSidecar`] from the given [`alloy_consensus::Block`].
    ///
    /// Returns [`ExecutionPayloadSidecar::none`] if the block does not contain any sidecar fields
    /// (pre-cancun): `requests_hash`, `parent_beacon_block_root`, `blob_versioned_hashes`.
    ///
    /// Note: This returns [`RequestOrHash::Hash`](alloy_eips::eip7685::RequestsOrHash::Hash) for
    /// the EIP-7685 requests.
    ///
    /// Bogota fields cannot be recovered from a block because inclusion-list transactions are not
    /// committed separately in the execution payload.
    pub fn from_block<T, H>(block: &Block<T, H>) -> Self
    where
        T: Transaction,
        H: BlockHeader,
    {
        let cancun =
            block.parent_beacon_block_root().map(|parent_beacon_block_root| CancunPayloadFields {
                parent_beacon_block_root,
                versioned_hashes: block.body.blob_versioned_hashes_iter().copied().collect(),
            });

        let prague = block.requests_hash().map(PraguePayloadFields::new);

        match (cancun, prague) {
            (Some(cancun), Some(prague)) => Self::v4(cancun, prague),
            (Some(cancun), None) => Self::v3(cancun),
            _ => Self::none(),
        }
    }

    /// Returns a new empty instance (pre-cancun, v1, v2).
    pub const fn none() -> Self {
        Self {
            cancun: MaybeCancunPayloadFields::none(),
            prague: MaybePraguePayloadFields::none(),
            bogota: MaybeBogotaPayloadFields::none(),
        }
    }

    /// Creates a new instance for cancun with the cancun fields for `engine_newPayloadV3`.
    pub fn v3(cancun: CancunPayloadFields) -> Self {
        Self {
            cancun: cancun.into(),
            prague: MaybePraguePayloadFields::none(),
            bogota: MaybeBogotaPayloadFields::none(),
        }
    }

    /// Creates a new instance post prague for `engine_newPayloadV4`.
    pub fn v4(cancun: CancunPayloadFields, prague: PraguePayloadFields) -> Self {
        Self {
            cancun: cancun.into(),
            prague: prague.into(),
            bogota: MaybeBogotaPayloadFields::none(),
        }
    }

    /// Creates a new instance post Bogota for `engine_newPayloadV6`.
    pub fn v6(
        cancun: CancunPayloadFields,
        prague: PraguePayloadFields,
        bogota: BogotaPayloadFields,
    ) -> Self {
        Self { cancun: cancun.into(), prague: prague.into(), bogota: bogota.into() }
    }

    /// Sets the EIP-7805 inclusion-list transactions.
    pub fn with_inclusion_list(mut self, inclusion_list_transactions: Vec<Bytes>) -> Self {
        self.bogota = BogotaPayloadFields::new(inclusion_list_transactions).into();
        self
    }

    /// Returns a reference to the [`CancunPayloadFields`].
    pub const fn cancun(&self) -> Option<&CancunPayloadFields> {
        self.cancun.as_ref()
    }

    /// Consumes the type and returns the [`CancunPayloadFields`]
    pub fn into_cancun(self) -> Option<CancunPayloadFields> {
        self.cancun.into_inner()
    }

    /// Returns a reference to the [`PraguePayloadFields`].
    pub const fn prague(&self) -> Option<&PraguePayloadFields> {
        self.prague.as_ref()
    }

    /// Consumes the type and returns the [`PraguePayloadFields`].
    pub fn into_prague(self) -> Option<PraguePayloadFields> {
        self.prague.into_inner()
    }

    /// Returns a reference to the [`BogotaPayloadFields`].
    pub const fn bogota(&self) -> Option<&BogotaPayloadFields> {
        self.bogota.as_ref()
    }

    /// Consumes the type and returns the [`BogotaPayloadFields`].
    pub fn into_bogota(self) -> Option<BogotaPayloadFields> {
        self.bogota.into_inner()
    }

    /// Returns the parent beacon block root, if any.
    pub fn parent_beacon_block_root(&self) -> Option<B256> {
        self.cancun.parent_beacon_block_root()
    }

    /// Returns the blob versioned hashes, if any.
    pub fn versioned_hashes(&self) -> Option<&Vec<B256>> {
        self.cancun.versioned_hashes()
    }

    /// Returns the EIP-7685 requests
    ///
    /// Note: if the [`PraguePayloadFields`] only contains the requests hash this will return
    /// `None`.
    pub fn requests(&self) -> Option<&Requests> {
        self.prague.requests()
    }

    /// Calculates or retrieves the requests hash.
    ///
    /// - If the `prague` field contains a list of requests, it calculates the requests hash
    ///   dynamically.
    /// - If it contains a precomputed hash (used for testing), it returns that hash directly.
    pub fn requests_hash(&self) -> Option<B256> {
        self.prague.requests_hash()
    }

    /// Returns the EIP-7805 inclusion-list transactions, if any.
    pub fn inclusion_list_transactions(&self) -> Option<&Vec<Bytes>> {
        self.bogota.inclusion_list_transactions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_inclusion_list() {
        use alloy_consensus::{BlockBody, Header, TxEnvelope};

        let block: Block<TxEnvelope> = Block::new(Header::default(), BlockBody::default());
        let transactions = vec![Bytes::from_static(&[0x01, 0x02])];
        let sidecar =
            ExecutionPayloadSidecar::from_block(&block).with_inclusion_list(transactions.clone());

        assert_eq!(sidecar.inclusion_list_transactions(), Some(&transactions));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_sidecar_without_bogota_fields() {
        let legacy = r#"{"cancun":{"fields":null},"prague":{"fields":null}}"#;
        let sidecar: ExecutionPayloadSidecar = serde_json::from_str(legacy).unwrap();

        assert!(sidecar.bogota().is_none());
        assert_eq!(serde_json::to_string(&sidecar).unwrap(), legacy);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_sidecar_with_bogota_fields() {
        let transactions = vec![Bytes::from_static(&[0x01, 0x02])];
        let sidecar = ExecutionPayloadSidecar::v6(
            CancunPayloadFields::default(),
            PraguePayloadFields::default(),
            BogotaPayloadFields::new(transactions.clone()),
        );

        let encoded = serde_json::to_string(&sidecar).unwrap();
        let decoded: ExecutionPayloadSidecar = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.inclusion_list_transactions(), Some(&transactions));
    }
}
