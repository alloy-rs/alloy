//! Contains consolidation types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251): Increase the MAX_EFFECTIVE_BALANCE

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// The address for the EIP-7251 consolidation requests contract:
/// `0x00b42dbF2194e931E80326D950320f7d9Dbeac02`
pub const CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS: Address =
    address!("00706203067988Ab3E2A2ab626EdCD6f28bDBbbb");

/// The code for the EIP-7251 consolidation requests contract.
pub static CONSOLIDATION_REQUEST_PREDEPLOY_CODE: Bytes = bytes!("3373fffffffffffffffffffffffffffffffffffffffe1460a8573615156028575f545f5260205ff35b36606014156101555760115f54600182026001905f5b5f82111560595781019083028483029004916001019190603e565b90939004341061015557600154600101600155600354806004026004013381556001015f358155600101602035815560010160403590553360601b5f5260605f60143760745fa0600101600355005b6003546002548082038060011160bc575060015b5f5b8181146101025780607402838201600402600401805490600101805490600101805490600101549260601b84529083601401528260340152906054015260010160be565b9101809214610114579060025561011f565b90505f6002555f6003555b5f548061049d141561012e57505f5b6001546001828201116101435750505f610149565b01600190035b5f555f6001556074025ff35b5f5ffd");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for consolidation requests.
pub const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;

/// This structure maps onto the consolidation request object from [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ConsolidationRequest {
    /// Source address
    pub source_address: Address,
    /// Source public key
    pub source_pubkey: FixedBytes<48>,
    /// Target public key
    pub target_pubkey: FixedBytes<48>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use alloy_rlp::{Decodable, Encodable};
    use core::str::FromStr;

    #[test]
    fn test_encode_decode_consolidation_request_roundtrip() {
        // Generate and test multiple cases in a single pass using an iterator.
        (0..4).for_each(|_| {
            let original_request = ConsolidationRequest {
                source_address: Address::random(),
                source_pubkey: FixedBytes::<48>::random(),
                target_pubkey: FixedBytes::<48>::random(),
            };

            // Encode the request
            let mut buf = Vec::new();
            original_request.encode(&mut buf);

            // Decode the request and assert equality
            let decoded_request =
                ConsolidationRequest::decode(&mut &buf[..]).expect("Failed to decode request");
            assert_eq!(original_request, decoded_request);
        });
    }

    #[test]
    fn test_serde_consolidation_request() {
        // Sample JSON input representing a consolidation request
        let json_data = r#"{
            "sourceAddress":"0x007eABCA654E67103dF02f49EbdC5f6Cd9387a07",
            "sourcePubkey":"0xb13ff174911d0137e5f2b739fbf172b22cba35a037ef1edb03683b75c9abf5b271f8d48ad279cc89c7fae91db631c1e7",
            "targetPubkey":"0xd0e5be6b709f2dc02a49f6e37e0d03b7d832b79b0db1c8bbfd5b81b8e57b79a1282fb99a671b4629a0e0bfffa7cf6d4f"
        }"#;

        // Deserialize the JSON into a ConsolidationRequest struct
        let consolidation_request: ConsolidationRequest =
            serde_json::from_str(json_data).expect("Failed to deserialize");

        // Verify the deserialized content
        assert_eq!(
            consolidation_request.source_address,
            Address::from_str("0x007eABCA654E67103dF02f49EbdC5f6Cd9387a07").unwrap()
        );
        assert_eq!(
            consolidation_request.source_pubkey,
            FixedBytes::<48>::from(hex!("b13ff174911d0137e5f2b739fbf172b22cba35a037ef1edb03683b75c9abf5b271f8d48ad279cc89c7fae91db631c1e7"))
        );
        assert_eq!(
            consolidation_request.target_pubkey,
            FixedBytes::<48>::from(hex!("d0e5be6b709f2dc02a49f6e37e0d03b7d832b79b0db1c8bbfd5b81b8e57b79a1282fb99a671b4629a0e0bfffa7cf6d4f"))
        );

        // Serialize the struct back into JSON
        let serialized_json =
            serde_json::to_string(&consolidation_request).expect("Failed to serialize");

        // Check if the serialized JSON matches the expected JSON structure
        let expected_json = r#"{"sourceAddress":"0x007eabca654e67103df02f49ebdc5f6cd9387a07","sourcePubkey":"0xb13ff174911d0137e5f2b739fbf172b22cba35a037ef1edb03683b75c9abf5b271f8d48ad279cc89c7fae91db631c1e7","targetPubkey":"0xd0e5be6b709f2dc02a49f6e37e0d03b7d832b79b0db1c8bbfd5b81b8e57b79a1282fb99a671b4629a0e0bfffa7cf6d4f"}"#;
        assert_eq!(serialized_json, expected_json);
    }
}
