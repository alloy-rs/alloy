//! Contains types related to the Bogota hardfork that will be used by RPC to communicate with the
//! beacon consensus engine.

use alloc::vec::Vec;
use alloy_primitives::Bytes;

/// Maximum byte length of an inclusion list.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/main/src/engine/bogota.md#constants>
pub const MAX_BYTES_PER_INCLUSION_LIST: u64 = 8192;

/// Fields introduced in `engine_newPayloadV6` that are not present in the `ExecutionPayload` RPC
/// object.
///
/// See also:
/// <https://github.com/ethereum/execution-apis/blob/main/src/engine/bogota.md#engine_newpayloadv6>
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BogotaPayloadFields {
    /// EIP-7805 inclusion-list transactions.
    pub inclusion_list_transactions: Vec<Bytes>,
}

impl BogotaPayloadFields {
    /// Returns a new [`BogotaPayloadFields`] instance.
    pub const fn new(inclusion_list_transactions: Vec<Bytes>) -> Self {
        Self { inclusion_list_transactions }
    }
}

impl From<Vec<Bytes>> for BogotaPayloadFields {
    fn from(inclusion_list_transactions: Vec<Bytes>) -> Self {
        Self::new(inclusion_list_transactions)
    }
}

/// A container type for [`BogotaPayloadFields`] that may or may not be present.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct MaybeBogotaPayloadFields {
    fields: Option<BogotaPayloadFields>,
}

impl MaybeBogotaPayloadFields {
    /// Returns a new [`MaybeBogotaPayloadFields`] with no Bogota fields.
    pub const fn none() -> Self {
        Self { fields: None }
    }

    /// Consumes `self` and returns the contained [`BogotaPayloadFields`], if present.
    pub fn into_inner(self) -> Option<BogotaPayloadFields> {
        self.fields
    }

    /// Returns the inclusion-list transactions, if any.
    pub fn inclusion_list_transactions(&self) -> Option<&Vec<Bytes>> {
        self.fields.as_ref().map(|fields| &fields.inclusion_list_transactions)
    }

    /// Returns a reference to the inner fields.
    pub const fn as_ref(&self) -> Option<&BogotaPayloadFields> {
        self.fields.as_ref()
    }
}

impl From<BogotaPayloadFields> for MaybeBogotaPayloadFields {
    fn from(fields: BogotaPayloadFields) -> Self {
        Self { fields: Some(fields) }
    }
}

impl From<Option<BogotaPayloadFields>> for MaybeBogotaPayloadFields {
    fn from(fields: Option<BogotaPayloadFields>) -> Self {
        Self { fields }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bogota_payload_fields_conversions() {
        let transactions = vec![Bytes::from_static(&[0x01, 0x02])];
        let fields: BogotaPayloadFields = transactions.clone().into();
        assert_eq!(fields.inclusion_list_transactions, transactions);

        let maybe_fields: MaybeBogotaPayloadFields = fields.into();
        assert_eq!(maybe_fields.inclusion_list_transactions(), Some(&transactions));
    }
}
