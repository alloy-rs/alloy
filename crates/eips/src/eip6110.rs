//! Contains Deposit request types, first introduced in the [Prague hardfork](https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md).
//!
//! See also [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110): Supply validator deposits on chain

use alloy_primitives::{address, Address, FixedBytes, B256};

#[cfg(feature = "sol-types")]
use alloc::vec::Vec;
#[cfg(feature = "sol-types")]
use alloy_primitives::Log;
#[cfg(feature = "sol-types")]
use alloy_sol_types::SolEvent;

/// Mainnet deposit contract address.
pub const MAINNET_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("0x00000000219ab540356cbb839cbe05303d7705fa");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for deposit requests.
pub const DEPOSIT_REQUEST_TYPE: u8 = 0x00;

/// The [EIP-6110 Consensus Specs](https://github.com/ethereum/consensus-specs/blob/2660af05390aa61f06142e1c6311a3a3c633f720/specs/_features/eip6110/beacon-chain.md#constants) defined maximum payload size.
pub const MAX_DEPOSIT_RECEIPTS_PER_PAYLOAD: usize = 8192;

/// This structure maps onto the deposit object from [EIP-6110](https://eips.ethereum.org/EIPS/eip-6110).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DepositRequest {
    /// Validator public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount of ether deposited in gwei
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::displayfromstr"))]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
    /// Deposit index
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::displayfromstr"))]
    pub index: u64,
}

#[cfg(feature = "sol-types")]
alloy_sol_types::sol! {
    /// Event emitted by the deposit contract when accepting a validator deposit.
    event DepositEvent(
        bytes pubkey,
        bytes withdrawal_credentials,
        bytes amount,
        bytes signature,
        bytes index
    );
}

/// Appends a decoded deposit event in EIP-6110 request encoding.
///
/// The event fields are concatenated in order, preserving the little-endian amount and index
/// emitted by the deposit contract. The EIP-7685 request type byte is not included.
/// Field lengths are assumed to have been validated by the deposit contract.
#[cfg(feature = "sol-types")]
pub fn accumulate_deposit_from_log(log: &Log<DepositEvent>, out: &mut Vec<u8>) {
    out.reserve(48 + 32 + 8 + 96 + 8);
    out.extend_from_slice(&log.pubkey);
    out.extend_from_slice(&log.withdrawal_credentials);
    out.extend_from_slice(&log.amount);
    out.extend_from_slice(&log.signature);
    out.extend_from_slice(&log.index);
}

/// Decodes deposit events from `address` and appends their EIP-6110 request bytes to `out`.
///
/// Logs from other addresses or with a different first topic are ignored. Matching events are
/// appended in input order, without an EIP-7685 request type byte. Callers can flatten receipt
/// logs to accumulate all deposits in a block.
///
/// Returns an error if a matching event cannot be ABI-decoded. Deposits appended before the
/// failing log remain in `out`.
#[cfg(feature = "sol-types")]
pub fn accumulate_deposits_from_logs<'a>(
    address: Address,
    logs: impl IntoIterator<Item = &'a Log>,
    out: &mut Vec<u8>,
) -> Result<(), alloy_sol_types::Error> {
    for log in logs {
        if log.address != address || log.topics().first() != Some(&DepositEvent::SIGNATURE_HASH) {
            continue;
        }
        accumulate_deposit_from_log(&DepositEvent::decode_log(log)?, out);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;

    #[test]
    #[cfg(feature = "serde")]
    fn test_serde_deposit_request() {
        // Sample JSON input representing a deposit request
        let json_data = r#"{"pubkey":"0x8e01a8f21bdc38991ada53ca86d6c78d874675a450a38431cc6aa0f12d5661e344784c56c8a211f7025224d1303ee801","withdrawal_credentials":"0x010000000000000000000000af6df504f08ddf582d604d2f0a593bc153c25dbd","amount":"18112749083033600","signature":"0xb65f3db79405544528d6d92040282f29171f4ff6e5abb2d59f9ee1f1254aced2a7000f87bc2684f543e913a7cc1007ea0e97289b349c553eecdf253cd3ef5814088ba3d4ac286f2634dac3d026d9a01e4c166dc75e249d626a0f1c180dab75ce","index":"13343631333247680512"}"#;

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

#[cfg(all(test, feature = "sol-types"))]
mod log_tests {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{b256, Bytes};

    fn deposit(index: u64) -> Log<DepositEvent> {
        Log {
            address: MAINNET_DEPOSIT_CONTRACT_ADDRESS,
            data: DepositEvent {
                pubkey: vec![0x11; 48].into(),
                withdrawal_credentials: vec![0x22; 32].into(),
                amount: 32_000_000_000u64.to_le_bytes().into(),
                signature: vec![0x33; 96].into(),
                index: index.to_le_bytes().into(),
            },
        }
    }

    #[test]
    fn deposit_request_encoding() {
        assert_eq!(
            DepositEvent::SIGNATURE_HASH,
            b256!("649bbc62d0e31342afea4e5cd82d4049e7e1ee912fc0889aa790803be39038c5")
        );
        let mut out = vec![DEPOSIT_REQUEST_TYPE];
        accumulate_deposit_from_log(&deposit(0x0102030405060708), &mut out);
        let mut expected = vec![DEPOSIT_REQUEST_TYPE];
        expected.extend_from_slice(&[0x11; 48]);
        expected.extend_from_slice(&[0x22; 32]);
        expected.extend_from_slice(&[0x00, 0x40, 0x59, 0x73, 0x07, 0x00, 0x00, 0x00]);
        expected.extend_from_slice(&[0x33; 96]);
        expected.extend_from_slice(&[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(out, expected);
    }

    #[test]
    fn filters_logs_and_preserves_order() {
        let address = Address::with_last_byte(0x42);
        let mut first = DepositEvent::encode_log(&deposit(7));
        first.address = address;
        let mut second = DepositEvent::encode_log(&deposit(8));
        second.address = address;
        let wrong_address = DepositEvent::encode_log(&deposit(9));
        let wrong_topic = Log::new_unchecked(address, vec![B256::ZERO], Bytes::new());
        let no_topics = Log::new_unchecked(address, vec![], Bytes::new());
        let logs = [wrong_address, first, wrong_topic, no_topics, second];
        let mut out = vec![0xff];
        accumulate_deposits_from_logs(address, &logs, &mut out).unwrap();
        let mut expected = vec![0xff];
        accumulate_deposit_from_log(&deposit(7), &mut expected);
        accumulate_deposit_from_log(&deposit(8), &mut expected);
        assert_eq!(out, expected);
    }

    #[test]
    fn rejects_malformed_matching_log() {
        let address = MAINNET_DEPOSIT_CONTRACT_ADDRESS;
        let mut malformed =
            Log::new_unchecked(Address::ZERO, vec![DepositEvent::SIGNATURE_HASH], Bytes::new());
        let mut out = Vec::new();
        accumulate_deposits_from_logs(address, [&malformed], &mut out).unwrap();
        assert!(out.is_empty());

        malformed.address = address;
        let valid = DepositEvent::encode_log(&deposit(7));
        assert!(accumulate_deposits_from_logs(address, [&valid, &malformed], &mut out).is_err());
        let mut expected = Vec::new();
        accumulate_deposit_from_log(&deposit(7), &mut expected);
        assert_eq!(out, expected);
    }
}
