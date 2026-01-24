//! Beacon block header types.
//!
//! See also <https://ethereum.github.io/beacon-APIs/#/Beacon/getBlockHeaders>

use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// The response to a request for beacon block headers: `getBlockHeaders`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HeadersResponse {
    /// True if the response references an unverified execution payload. Optimistic information may
    /// be invalidated at a later time. If the field is not present, assume the False value.
    pub execution_optimistic: bool,
    /// True if the response references the finalized history of the chain, as determined by fork
    /// choice. If the field is not present, additional calls are necessary to compare the epoch of
    /// the requested information with the finalized checkpoint.
    pub finalized: bool,
    /// Container for the header data.
    pub data: Vec<HeaderData>,
}

/// The response to a request for a __single__ beacon block header: `headers/{id}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HeaderResponse {
    /// True if the response references an unverified execution payload. Optimistic information may
    /// be invalidated at a later time. If the field is not present, assume the False value.
    pub execution_optimistic: bool,
    /// True if the response references the finalized history of the chain, as determined by fork
    /// choice. If the field is not present, additional calls are necessary to compare the epoch of
    /// the requested information with the finalized checkpoint.
    pub finalized: bool,
    /// Container for the header data.
    pub data: HeaderData,
}

/// Container type for a beacon block header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HeaderData {
    /// root hash of the block
    pub root: B256,
    /// Whether the block is part of the canonical chain
    pub canonical: bool,
    /// The `SignedBeaconBlockHeader` object envelope from the CL spec.
    pub header: Header,
}

/// [BeaconBlockHeader] with a signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Header {
    /// The [`BeaconBlockHeader`] object from the CL spec.
    pub message: BeaconBlockHeader,
    /// The signature associated with the [`BeaconBlockHeader`].
    pub signature: Bytes,
}

/// The header of a beacon block.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct BeaconBlockHeader {
    /// The slot to which this block corresponds.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// Index of validator in validator registry.
    #[serde_as(as = "DisplayFromStr")]
    pub proposer_index: u64,
    /// The signing Merkle root of the parent BeaconBlock.
    pub parent_root: B256,
    /// The tree hash Merkle root of the BeaconState for the BeaconBlock.
    pub state_root: B256,
    /// The tree hash Merkle root of the BeaconBlockBody for the BeaconBlock
    pub body_root: B256,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_headers_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": false,
            "data": [
                {
                    "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                    "canonical": true,
                    "header": {
                        "message": {
                            "slot": "1",
                            "proposer_index": "1",
                            "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                            "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                            "body_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                        },
                        "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
                    }
                }
            ]
        }"#;
        let _header_response: HeadersResponse = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn serde_header_response() {
        let s = r#"{
            "execution_optimistic": false,
            "finalized": false,
            "data": {
                "root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                "canonical": true,
                "header": {
                    "message": {
                        "slot": "1",
                        "proposer_index": "1",
                        "parent_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                        "state_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2",
                        "body_root": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884560367e8208d920f2"
                    },
                    "signature": "0x1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505cc411d61252fb6cb3fa0017b679f8bb2305b26a285fa2737f175668d0dff91cc1b66ac1fb663c9bc59509846d6ec05345bd908eda73e670af888da41af171505"
                }
            }
        }"#;
        let _header_response: HeaderResponse = serde_json::from_str(s).unwrap();
    }

    #[cfg(feature = "ssz")]
    mod ssz_tests {
        use super::*;
        use ssz::{Decode, Encode};

        #[test]
        fn ssz_roundtrip_beacon_block_header() {
            let header = BeaconBlockHeader {
                slot: 12345,
                proposer_index: 678,
                parent_root: B256::repeat_byte(0x11),
                state_root: B256::repeat_byte(0x22),
                body_root: B256::repeat_byte(0x33),
            };
            let encoded = header.as_ssz_bytes();
            let decoded = BeaconBlockHeader::from_ssz_bytes(&encoded).unwrap();
            assert_eq!(header, decoded);
        }
    }
}
