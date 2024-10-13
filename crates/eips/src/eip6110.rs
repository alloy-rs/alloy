//! Contains Deposit types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110): Supply validator deposits on chain
//!
//! Provides validator deposits as a list of deposit operations added to the Execution Layer block.

use alloy_primitives::{address, Address, FixedBytes, B256};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// Mainnet deposit contract address.
pub const MAINNET_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("00000000219ab540356cbb839cbe05303d7705fa");

/// This structure maps onto the deposit object from [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DepositRequest {
    /// Validator public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount of ether deposited in gwei
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
    /// Deposit index
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use alloy_rlp::{Decodable, Encodable};

    #[test]
    fn test_encode_decode_request_roundtrip() {
        // Define multiple test cases as tuples containing the test data
        let test_cases = vec![
            (
                // https://etherscan.io/tx/0xab9e0b47767c6172f49f691e5fd96cb257c17f2d39cf64742d71e5435308403c#eventlog
                FixedBytes::<48>::from(hex!("8E01A8F21BDC38991ADA53CA86D6C78D874675A450A38431CC6AA0F12D5661E344784C56C8A211F7025224D1303EE801")),
                B256::from(hex!("010000000000000000000000AF6DF504F08DDF582D604D2F0A593BC153C25DBD")),
                0x0040597307000000u64,
                FixedBytes::<96>::from(hex!("B65F3DB79405544528D6D92040282F29171F4FF6E5ABB2D59F9EE1F1254ACED2A7000F87BC2684F543E913A7CC1007EA0E97289B349C553EECDF253CD3EF5814088BA3D4AC286F2634DAC3D026D9A01E4C166DC75E249D626A0F1C180DAB75CE")),
                0xB92E1A0000000000u64,
            ),
            // https://etherscan.io/tx/0x756a8aba9f8df9fba33519bc1ec1ad2251507f66ef65cb15eb0a80ddfd0bcbef#eventlog
            (
                FixedBytes::<48>::from(hex!("85BA6057EA5100DCB0B347D545BE0B688B2AD10A029C24A9D653F18BA3AC743B3CF8E022AE487AD8C5670D9C20D101D8")),
                B256::from(hex!("010000000000000000000000E839A3E9EFB32C6A56AB7128E51056585275506C")),
                0x0040597307000000u64,
                FixedBytes::<96>::from(hex!("8F685E17FC36B8DE5EF6E81523995139EF59280F7D25D0C422C7BDF573217F8127B2425B87E414443430B4EE05EE5ABE19EEDB9B239BA1354FBB8133C5B068FA7B278296856F7C7592F1AF9332762AB4389B9FC224D32E209077368AE3CED710")),
                0xDD2A1A0000000000u64,
            ),
            (
                // https://etherscan.io/tx/0xa5759378809a22bf8072de41c155e176a74a08323e94e4167ee2692887e83859#eventlog
                FixedBytes::<48>::from(hex!("A3151E4E6BE6A4002249331B60EF426F6CDE5C33B27C9F14FC6639E6888A10F54C4A44AEE7AB0690BF09A89BDC00237C")),
                B256::from(hex!("01000000000000000000000006676E8584342CC8B6052CFDF381C3A281F00AC8")),
                0x0040597307000000u64,
                FixedBytes::<96>::from(hex!("A3C85DF60DF11200166D49A055E96C4659D37AC630CDC6E5C3EE039478E5B558F50F390D306249CEC66ACAA09011B85300434FFB566FE599E3E1596162BC3BBCE7BEE9122DAEBF7D1F42124C0FF00BE6EE8B79E0F436044337148EB061E0B163")),
                0x85A6130000000000u64,
            ),
            (
                // https://etherscan.io/tx/0x132bb7c90069d9699a84cdd041ddbe7a5cc42b2d26b604a8ac282aa5c17ed218#eventlog
                FixedBytes::<48>::from(hex!("AAE673FEE94E4552CFEC432DCDDF46D1F613DD48E3DFB36179C973B73BCFDE5C463C5B719DD916DA8DA3981BFD0BAE29")),
                B256::from(hex!("010000000000000000000000A8C62111E4652B07110A0FC81816303C42632F64")),
                0x0040597307000000u64,
                FixedBytes::<96>::from(hex!("B6BA4C2CA28E46BA0DD8757EBDD52BC3609DA3BCF17BCCEEDD181630797F2A51CA2D8B4BFBA8639074276ABA6A4B7316106AB9F1642BAB0D9A8058211F366BBEB6E42CB1C56D17155A84B2C32F61F772C41749134665CBD7EC43122691527050")),
                0xC9A8130000000000u64,
            ),
            (
                // https://etherscan.io/tx/0xc97358e047333933278b8ab08ec4cfff8f6cf4028e2c3d877a5a89bd9f7303c9#eventlog
                FixedBytes::<48>::from(hex!("A3CF35BEC7827E666C591BE49336B19FDE0F6ADA5F7139CFEEB719F372B200EDAC11FE73EDE7BE5B4A17E43BF055C58A")),
                B256::from(hex!("0100000000000000000000008B7F4F725AE240D9B28D8129D35E6580D1251852")),
                0x0040597307000000u64,
                FixedBytes::<96>::from(hex!("B9844B35D0831E22DF5E8374FF6C405F98DED278E813EBF9F1A61CFC08F6019D99E3C4B975BFDE999DC2AAC8EA99C540112FFAB1557CE0ADD9D80E0F91EB2D370F3220DA04D96F6256A07288CEFDB2F27C2C3DDE26B49EBAF5801E99FDCA095D")),
                0x62301A0000000000u64,
            ),
        ];

        // Iterate over each test case
        for (pubkey, withdrawal_credentials, amount, signature, index) in test_cases {
            let original_request =
                DepositRequest { pubkey, withdrawal_credentials, amount, signature, index };

            // Encode the request
            let mut buf = Vec::new();
            original_request.encode(&mut buf);

            // Decode the request
            let decoded_request =
                DepositRequest::decode(&mut &buf[..]).expect("Failed to decode request");

            // Ensure the encoded and then decoded request matches the original
            assert_eq!(original_request, decoded_request);
        }
    }

    #[test]
    fn test_serde_deposit_request() {
        // Sample JSON input representing a deposit request
        let json_data = r#"{"pubkey":"0x8e01a8f21bdc38991ada53ca86d6c78d874675a450a38431cc6aa0f12d5661e344784c56c8a211f7025224d1303ee801","withdrawalCredentials":"0x010000000000000000000000af6df504f08ddf582d604d2f0a593bc153c25dbd","amount":"0x40597307000000","signature":"0xb65f3db79405544528d6d92040282f29171f4ff6e5abb2d59f9ee1f1254aced2a7000f87bc2684f543e913a7cc1007ea0e97289b349c553eecdf253cd3ef5814088ba3d4ac286f2634dac3d026d9a01e4c166dc75e249d626a0f1c180dab75ce","index":"0xb92e1a0000000000"}"#;

        // Deserialize the JSON into a DepositRequest struct
        let deposit_request: DepositRequest =
            serde_json::from_str(json_data).expect("Failed to deserialize");

        // Verify the deserialized content
        assert_eq!(
            deposit_request.pubkey,
            FixedBytes::<48>::from(hex!("8E01A8F21BDC38991ADA53CA86D6C78D874675A450A38431CC6AA0F12D5661E344784C56C8A211F7025224D1303EE801"))
        );
        assert_eq!(
            deposit_request.withdrawal_credentials,
            B256::from(hex!("010000000000000000000000AF6DF504F08DDF582D604D2F0A593BC153C25DBD"))
        );
        assert_eq!(deposit_request.amount, 0x0040597307000000u64);
        assert_eq!(
            deposit_request.signature,
            FixedBytes::<96>::from(hex!("B65F3DB79405544528D6D92040282F29171F4FF6E5ABB2D59F9EE1F1254ACED2A7000F87BC2684F543E913A7CC1007EA0E97289B349C553EECDF253CD3EF5814088BA3D4AC286F2634DAC3D026D9A01E4C166DC75E249D626A0F1C180DAB75CE"))
        );
        assert_eq!(deposit_request.index, 0xB92E1A0000000000u64);

        // Serialize the struct back into JSON
        let serialized_json = serde_json::to_string(&deposit_request).expect("Failed to serialize");

        // Check if the serialized JSON matches the expected JSON structure
        assert_eq!(serialized_json, json_data);
    }
}
