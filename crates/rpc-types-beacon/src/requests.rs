#[cfg(feature = "ssz")]
use alloy_eips::eip7685::Requests;
use alloy_eips::{
    eip6110::DepositRequest,
    eip7002::WithdrawalRequest,
    eip7251::ConsolidationRequest,
    eip8282::{BuilderDepositRequest, BuilderExitRequest},
};
use serde::{Deserialize, Serialize};

/// An Electra-compatible execution requests payload.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Decode, ssz_derive::Encode))]
pub struct ExecutionRequestsV4 {
    /// The requested deposits.
    pub deposits: Vec<DepositRequest>,
    /// The requested withdrawals.
    pub withdrawals: Vec<WithdrawalRequest>,
    /// The requested consolidations.
    pub consolidations: Vec<ConsolidationRequest>,
}

impl<'de> Deserialize<'de> for ExecutionRequestsV4 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(default)]
            deposits: Option<Vec<DepositRequest>>,
            #[serde(default)]
            withdrawals: Option<Vec<WithdrawalRequest>>,
            #[serde(default)]
            consolidations: Option<Vec<ConsolidationRequest>>,
        }

        let helper = Helper::deserialize(deserializer)?;

        Ok(Self {
            deposits: helper.deposits.unwrap_or_default(),
            withdrawals: helper.withdrawals.unwrap_or_default(),
            consolidations: helper.consolidations.unwrap_or_default(),
        })
    }
}

impl ExecutionRequestsV4 {
    /// Convert the [ExecutionRequestsV4] into a [Requests].
    #[cfg(feature = "ssz")]
    pub fn to_requests(&self) -> Requests {
        self.into()
    }
}

/// An Amsterdam-compatible execution requests payload.
///
/// Extends [`ExecutionRequestsV4`] with the
/// [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282) builder deposit and exit requests. Every
/// field is optional on deserialization, so a body that predates EIP-8282 still parses, with the
/// two builder lists left empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Decode, ssz_derive::Encode))]
pub struct ExecutionRequestsV5 {
    /// The requested deposits.
    pub deposits: Vec<DepositRequest>,
    /// The requested withdrawals.
    pub withdrawals: Vec<WithdrawalRequest>,
    /// The requested consolidations.
    pub consolidations: Vec<ConsolidationRequest>,
    /// The requested builder deposits.
    pub builder_deposits: Vec<BuilderDepositRequest>,
    /// The requested builder exits.
    pub builder_exits: Vec<BuilderExitRequest>,
}

impl<'de> Deserialize<'de> for ExecutionRequestsV5 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(default)]
            deposits: Option<Vec<DepositRequest>>,
            #[serde(default)]
            withdrawals: Option<Vec<WithdrawalRequest>>,
            #[serde(default)]
            consolidations: Option<Vec<ConsolidationRequest>>,
            #[serde(default)]
            builder_deposits: Option<Vec<BuilderDepositRequest>>,
            #[serde(default)]
            builder_exits: Option<Vec<BuilderExitRequest>>,
        }

        let helper = Helper::deserialize(deserializer)?;

        Ok(Self {
            deposits: helper.deposits.unwrap_or_default(),
            withdrawals: helper.withdrawals.unwrap_or_default(),
            consolidations: helper.consolidations.unwrap_or_default(),
            builder_deposits: helper.builder_deposits.unwrap_or_default(),
            builder_exits: helper.builder_exits.unwrap_or_default(),
        })
    }
}

impl ExecutionRequestsV5 {
    /// Convert the [ExecutionRequestsV5] into a [Requests].
    #[cfg(feature = "ssz")]
    pub fn to_requests(&self) -> Requests {
        self.into()
    }
}

impl From<ExecutionRequestsV4> for ExecutionRequestsV5 {
    fn from(requests: ExecutionRequestsV4) -> Self {
        Self {
            deposits: requests.deposits,
            withdrawals: requests.withdrawals,
            consolidations: requests.consolidations,
            builder_deposits: Vec::new(),
            builder_exits: Vec::new(),
        }
    }
}

#[cfg(feature = "ssz")]
pub use ssz_requests_conversions::TryFromRequestsError;

#[cfg(feature = "ssz")]
mod ssz_requests_conversions {
    use super::*;
    use crate::requests::TryFromRequestsError::SszDecodeError;
    use alloy_eips::{
        eip6110::{DepositRequest, DEPOSIT_REQUEST_TYPE, MAX_DEPOSIT_RECEIPTS_PER_PAYLOAD},
        eip7002::{WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_BLOCK, WITHDRAWAL_REQUEST_TYPE},
        eip7251::{
            ConsolidationRequest, CONSOLIDATION_REQUEST_TYPE, MAX_CONSOLIDATION_REQUESTS_PER_BLOCK,
        },
        eip7685::Requests,
        eip8282::{
            BUILDER_DEPOSIT_REQUEST_TYPE, BUILDER_EXIT_REQUEST_TYPE,
            MAX_BUILDER_DEPOSIT_REQUESTS_PER_BLOCK, MAX_BUILDER_EXIT_REQUESTS_PER_BLOCK,
        },
    };
    use ssz::{Decode, DecodeError, Encode};

    fn parse_request_payload<T>(
        payload: &[u8],
        max_size: usize,
        request_type: u8,
    ) -> Result<Vec<T>, TryFromRequestsError>
    where
        T: Decode,
    {
        let list: Vec<T> =
            Vec::from_ssz_bytes(payload).map_err(|e| SszDecodeError(request_type, e))?;

        if list.len() > max_size {
            return Err(TryFromRequestsError::RequestPayloadSizeExceeded(request_type, list.len()));
        }

        Ok(list)
    }

    impl TryFrom<Requests> for ExecutionRequestsV4 {
        type Error = TryFromRequestsError;

        fn try_from(value: Requests) -> Result<Self, Self::Error> {
            Self::try_from(&value)
        }
    }

    impl TryFrom<&Requests> for ExecutionRequestsV4 {
        type Error = TryFromRequestsError;

        fn try_from(value: &Requests) -> Result<Self, Self::Error> {
            #[derive(Default)]
            struct RequestAccumulator {
                deposits: Vec<DepositRequest>,
                withdrawals: Vec<WithdrawalRequest>,
                consolidations: Vec<ConsolidationRequest>,
            }

            impl RequestAccumulator {
                fn accumulate(mut self, request: &[u8]) -> Result<Self, TryFromRequestsError> {
                    if request.is_empty() {
                        return Err(TryFromRequestsError::EmptyRequest);
                    }

                    let (request_type, payload) =
                        request.split_first().expect("already checked for empty");

                    match *request_type {
                        DEPOSIT_REQUEST_TYPE => {
                            self.deposits = parse_request_payload(
                                payload,
                                MAX_DEPOSIT_RECEIPTS_PER_PAYLOAD,
                                DEPOSIT_REQUEST_TYPE,
                            )?;
                        }
                        WITHDRAWAL_REQUEST_TYPE => {
                            self.withdrawals = parse_request_payload(
                                payload,
                                MAX_WITHDRAWAL_REQUESTS_PER_BLOCK,
                                WITHDRAWAL_REQUEST_TYPE,
                            )?;
                        }
                        CONSOLIDATION_REQUEST_TYPE => {
                            self.consolidations = parse_request_payload(
                                payload,
                                MAX_CONSOLIDATION_REQUESTS_PER_BLOCK,
                                CONSOLIDATION_REQUEST_TYPE,
                            )?;
                        }
                        unknown => return Err(TryFromRequestsError::UnknownRequestType(unknown)),
                    }

                    Ok(self)
                }
            }

            let accumulator = value
                .iter()
                .try_fold(RequestAccumulator::default(), |acc, request| acc.accumulate(request))?;

            Ok(Self {
                deposits: accumulator.deposits,
                withdrawals: accumulator.withdrawals,
                consolidations: accumulator.consolidations,
            })
        }
    }

    /// Errors possible converting a [Requests] to [ExecutionRequestsV4] or [ExecutionRequestsV5]
    #[derive(Debug, thiserror::Error)]
    pub enum TryFromRequestsError {
        /// One of the Bytes is empty.
        #[error("empty bytes in requests body")]
        EmptyRequest,
        /// Bytes prefix is not a known EIP-7685 request_type for the target version.
        #[error("unknown request_type prefix: {0}")]
        UnknownRequestType(u8),
        /// Remaining bytes could not be decoded as SSZ requests_data.
        #[error("ssz decode error for request_type {0}: {1:?}")]
        SszDecodeError(u8, DecodeError),
        /// Requests of request_type exceeds the size limit for that type
        #[error("requests_data payload for request_type {0} exceeds size limit {1}")]
        RequestPayloadSizeExceeded(u8, usize),
    }

    impl From<&ExecutionRequestsV4> for Requests {
        fn from(val: &ExecutionRequestsV4) -> Self {
            let deposit_bytes = val.deposits.as_ssz_bytes();
            let withdrawals_bytes = val.withdrawals.as_ssz_bytes();
            let consolidations_bytes = val.consolidations.as_ssz_bytes();

            let mut requests = Self::with_capacity(3);
            requests.push_request_with_type(DEPOSIT_REQUEST_TYPE, deposit_bytes);
            requests.push_request_with_type(WITHDRAWAL_REQUEST_TYPE, withdrawals_bytes);
            requests.push_request_with_type(CONSOLIDATION_REQUEST_TYPE, consolidations_bytes);
            requests
        }
    }

    impl TryFrom<Requests> for ExecutionRequestsV5 {
        type Error = TryFromRequestsError;

        fn try_from(value: Requests) -> Result<Self, Self::Error> {
            Self::try_from(&value)
        }
    }

    impl TryFrom<&Requests> for ExecutionRequestsV5 {
        type Error = TryFromRequestsError;

        /// Takes the two EIP-8282 request types out of the body and defers the rest to
        /// [`ExecutionRequestsV4`], which still rejects any type byte neither of us knows.
        fn try_from(value: &Requests) -> Result<Self, Self::Error> {
            let mut builder_deposits = Vec::new();
            let mut builder_exits = Vec::new();
            let mut rest = Vec::with_capacity(value.len());

            for request in value.iter() {
                let (request_type, payload) =
                    request.split_first().ok_or(TryFromRequestsError::EmptyRequest)?;

                match *request_type {
                    BUILDER_DEPOSIT_REQUEST_TYPE => {
                        builder_deposits = parse_request_payload(
                            payload,
                            MAX_BUILDER_DEPOSIT_REQUESTS_PER_BLOCK,
                            BUILDER_DEPOSIT_REQUEST_TYPE,
                        )?;
                    }
                    BUILDER_EXIT_REQUEST_TYPE => {
                        builder_exits = parse_request_payload(
                            payload,
                            MAX_BUILDER_EXIT_REQUESTS_PER_BLOCK,
                            BUILDER_EXIT_REQUEST_TYPE,
                        )?;
                    }
                    _ => rest.push(request.clone()),
                }
            }

            let electra = ExecutionRequestsV4::try_from(&Requests::from(rest))?;

            Ok(Self {
                deposits: electra.deposits,
                withdrawals: electra.withdrawals,
                consolidations: electra.consolidations,
                builder_deposits,
                builder_exits,
            })
        }
    }

    impl From<&ExecutionRequestsV5> for Requests {
        fn from(val: &ExecutionRequestsV5) -> Self {
            let deposit_bytes = val.deposits.as_ssz_bytes();
            let withdrawals_bytes = val.withdrawals.as_ssz_bytes();
            let consolidations_bytes = val.consolidations.as_ssz_bytes();
            let builder_deposit_bytes = val.builder_deposits.as_ssz_bytes();
            let builder_exit_bytes = val.builder_exits.as_ssz_bytes();

            let mut requests = Self::with_capacity(5);
            requests.push_request_with_type(DEPOSIT_REQUEST_TYPE, deposit_bytes);
            requests.push_request_with_type(WITHDRAWAL_REQUEST_TYPE, withdrawals_bytes);
            requests.push_request_with_type(CONSOLIDATION_REQUEST_TYPE, consolidations_bytes);
            requests.push_request_with_type(BUILDER_DEPOSIT_REQUEST_TYPE, builder_deposit_bytes);
            requests.push_request_with_type(BUILDER_EXIT_REQUEST_TYPE, builder_exit_bytes);
            requests
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use alloy_primitives::Bytes;
        use std::str::FromStr;
        #[test]
        fn test_from_requests() -> Result<(), TryFromRequestsError> {
            let original = Requests::new(vec![
                // Taken from: https://github.com/ensi321/execution-apis/blob/88c08d6104e9e8ae1d369c2b26c393a0df599e9a/src/engine/openrpc/methods/payload.yaml#L554-L556
                Bytes::from_str("0x0096a96086cff07df17668f35f7418ef8798079167e3f4f9b72ecde17b28226137cf454ab1dd20ef5d924786ab3483c2f9003f5102dabe0a27b1746098d1dc17a5d3fbd478759fea9287e4e419b3c3cef20100000000000000b1acdb2c4d3df3f1b8d3bfd33421660df358d84d78d16c4603551935f4b67643373e7eb63dcb16ec359be0ec41fee33b03a16e80745f2374ff1d3c352508ac5d857c6476d3c3bcf7e6ca37427c9209f17be3af5264c0e2132b3dd1156c28b4e9f000000000000000a5c85a60ba2905c215f6a12872e62b1ee037051364244043a5f639aa81b04a204c55e7cc851f29c7c183be253ea1510b001db70c485b6264692f26b8aeaab5b0c384180df8e2184a21a808a3ec8e86ca01000000000000009561731785b48cf1886412234531e4940064584463e96ac63a1a154320227e333fb51addc4a89b7e0d3f862d7c1fd4ea03bd8eb3d8806f1e7daf591cbbbb92b0beb74d13c01617f22c5026b4f9f9f294a8a7c32db895de3b01bee0132c9209e1f100000000000000").unwrap(),
                Bytes::from_str("0x01a94f5374fce5edbc8e2a8697c15331677e6ebf0b85103a5617937691dfeeb89b86a80d5dc9e3c9d3a1a0e7ce311e26e0bb732eabaa47ffa288f0d54de28209a62a7d29d0000000000000000000000000000000000000000000000000000010f698daeed734da114470da559bd4b4c7259e1f7952555241dcbc90cf194a2ef676fc6005f3672fada2a3645edb297a75530100000000000000").unwrap(),
                Bytes::from_str("0x02a94f5374fce5edbc8e2a8697c15331677e6ebf0b85103a5617937691dfeeb89b86a80d5dc9e3c9d3a1a0e7ce311e26e0bb732eabaa47ffa288f0d54de28209a62a7d29d098daeed734da114470da559bd4b4c7259e1f7952555241dcbc90cf194a2ef676fc6005f3672fada2a3645edb297a7553").unwrap(),
            ]);

            let requests = ExecutionRequestsV4::try_from(&original)?;
            assert_eq!(requests.deposits.len(), 2);
            assert_eq!(requests.withdrawals.len(), 2);
            assert_eq!(requests.consolidations.len(), 1);

            let round_trip: Requests = (&requests).into();
            assert_eq!(original, round_trip);
            Ok(())
        }

        #[test]
        fn test_from_requests_v5_builder_requests() -> Result<(), TryFromRequestsError> {
            let builder_deposit = BuilderDepositRequest {
                pubkey: alloy_primitives::FixedBytes::repeat_byte(0x11),
                withdrawal_credentials: alloy_primitives::B256::repeat_byte(0x22),
                amount: 91_000_000_000,
                signature: alloy_primitives::FixedBytes::repeat_byte(0x33),
            };
            let builder_exit = BuilderExitRequest {
                source_address: alloy_primitives::Address::repeat_byte(0x44),
                pubkey: alloy_primitives::FixedBytes::repeat_byte(0x55),
            };

            let original = Requests::new(vec![
                Bytes::from_iter(
                    core::iter::once(BUILDER_DEPOSIT_REQUEST_TYPE)
                        .chain(vec![builder_deposit].as_ssz_bytes()),
                ),
                Bytes::from_iter(
                    core::iter::once(BUILDER_EXIT_REQUEST_TYPE)
                        .chain(vec![builder_exit].as_ssz_bytes()),
                ),
            ]);

            let requests = ExecutionRequestsV5::try_from(&original)?;
            assert_eq!(requests.builder_deposits, vec![builder_deposit]);
            assert_eq!(requests.builder_exits, vec![builder_exit]);
            assert!(requests.deposits.is_empty());

            let round_trip: Requests = (&requests).into();
            assert_eq!(original, round_trip);

            // A builder request type is not valid in an Electra body.
            assert!(matches!(
                ExecutionRequestsV4::try_from(&original),
                Err(TryFromRequestsError::UnknownRequestType(BUILDER_DEPOSIT_REQUEST_TYPE))
            ));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserde_requests_v4() {
        let s = r#"{"deposits":null,"withdrawals":null,"consolidations":null}"#;
        let requests: ExecutionRequestsV4 = serde_json::from_str(s).unwrap();
        assert_eq!(requests, ExecutionRequestsV4::default());

        let s = r#"{"deposits":null,"withdrawals":null}"#;
        let requests: ExecutionRequestsV4 = serde_json::from_str(s).unwrap();
        assert_eq!(requests, ExecutionRequestsV4::default());

        let s = r#"{"deposits":[],"withdrawals":[],"consolidations":[]}"#;
        let requests: ExecutionRequestsV4 = serde_json::from_str(s).unwrap();
        assert_eq!(requests, ExecutionRequestsV4::default());
    }

    #[test]
    fn deserde_requests_v5() {
        // A body that predates EIP-8282 parses with empty builder requests.
        let s = r#"{"deposits":null,"withdrawals":null,"consolidations":null}"#;
        let requests: ExecutionRequestsV5 = serde_json::from_str(s).unwrap();
        assert_eq!(requests, ExecutionRequestsV5::default());

        let s = r#"{"deposits":[],"withdrawals":[],"consolidations":[],"builder_deposits":[],"builder_exits":[]}"#;
        let requests: ExecutionRequestsV5 = serde_json::from_str(s).unwrap();
        assert_eq!(requests, ExecutionRequestsV5::default());

        let s = r#"{"builder_deposits":[{"pubkey":"0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111","withdrawal_credentials":"0x2222222222222222222222222222222222222222222222222222222222222222","amount":"91000000000","signature":"0x333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333"}],"builder_exits":[{"source_address":"0x4444444444444444444444444444444444444444","pubkey":"0x555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555"}]}"#;
        let requests: ExecutionRequestsV5 = serde_json::from_str(s).unwrap();
        assert_eq!(requests.builder_deposits.len(), 1);
        assert_eq!(requests.builder_deposits[0].amount, 91_000_000_000);
        assert_eq!(requests.builder_exits.len(), 1);
    }
}
