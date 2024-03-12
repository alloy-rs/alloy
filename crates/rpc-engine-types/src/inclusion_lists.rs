//! Contains types related to the Inclusion lists that will be used by in the engine API RPC
//! definitions.

use crate::PayloadStatusEnum;
use alloy_primitives::Address;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::fmt;

/// This structure contains the result of processing an `engine_newInclusionListV1` RPC call.
///
/// From the spec:
///
/// ### InclusionListStatusV1
///
/// This structure contains the result of processing an inclusion list. The fields are encoded as
/// follows:
/// - `status`: `enum` - `"VALID" | "INVALID" | "SYNCING" | "ACCEPTED"`
/// - `validationError`: `String|null` - a message providing additional details on the validation
/// error if the payload is classified as `INVALID`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListStatusV1 {
    /// The status of the payload.
    #[serde(flatten)]
    pub status: PayloadStatusEnum,
}

impl InclusionListStatusV1 {
    /// Initializes a new inclusion list status.
    pub const fn new(status: PayloadStatusEnum) -> Self {
        Self { status }
    }

    /// Returns true if the payload status is syncing.
    pub const fn is_syncing(&self) -> bool {
        self.status.is_syncing()
    }

    /// Returns true if the payload status is valid.
    pub const fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    /// Returns true if the payload status is invalid.
    pub const fn is_invalid(&self) -> bool {
        self.status.is_invalid()
    }
}

impl fmt::Display for InclusionListStatusV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InclusionListStatusV1 {{ status: {} }}", self.status)
    }
}

impl Serialize for InclusionListStatusV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("status", self.status.as_str())?;
        map.serialize_entry("validationError", &self.status.validation_error())?;
        map.end()
    }
}

/// This structure contains the input to the `engine_newInclusionListV1` RPC call.
///
/// From the spec:
///
/// ### InclusionListEntryV1
///
/// - `address`: `DATA`, 20 bytes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListEntryV1 {
    /// The address of the inclusion list entry.
    pub address: Address,
}

impl fmt::Display for InclusionListEntryV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InclusionListEntryV1 {{ address: {} }}", self.address)
    }
}

/// This contains the configuration for the `engine_newInclusionListV1` RPC call.
///
/// From the spec:
///
/// ### InclusionListConfiguration
///
/// - `inclusionListMaxGas`: `QUANTITY`, 64 bits
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionListConfiguration {
    /// The maximum gas for the inclusion list.
    pub inclusion_list_max_gas: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex::FromHex;
    use serde_json::json;

    #[test]
    fn inclusion_list_status_v1_serialization() {
        let status = InclusionListStatusV1::new(PayloadStatusEnum::Valid);
        let json = json!({
            "status": "VALID",
            "validationError": null,
        });
        assert_eq!(serde_json::to_value(status).unwrap(), json);
    }

    #[test]
    fn inclusion_list_entry_v1_serialization() {
        let entry = InclusionListEntryV1 {
            address: Address::from_hex("0x0000000000000000000000000000000000000042").unwrap(),
        };
        let json = json!({
            "address": "0x0000000000000000000000000000000000000042",
        });
        assert_eq!(serde_json::to_value(entry).unwrap(), json);
    }

    #[test]
    fn inclusion_list_configuration_serialization() {
        let config = InclusionListConfiguration { inclusion_list_max_gas: 42 };
        let json = json!({
            "inclusionListMaxGas": 42,
        });
        assert_eq!(serde_json::to_value(config).unwrap(), json);
    }
}
