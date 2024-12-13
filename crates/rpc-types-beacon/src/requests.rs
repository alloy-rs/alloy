use alloy_eips::{
    eip6110::DepositRequest, eip7002::WithdrawalRequest, eip7251::ConsolidationRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Decode, ssz_derive::Encode))]
/// An Electra-compatible execution requests payload.
pub struct ExecutionRequestsV4 {
    /// The requested deposits.
    pub deposits: Vec<DepositRequest>,
    /// The requested withdrawals.
    pub withdrawals: Vec<WithdrawalRequest>,
    /// The requested consolidations.
    pub consolidations: Vec<ConsolidationRequest>,
}

#[cfg(feature = "ssz")]
pub use ssz_requests_conversions::TryFromRequestsError;

#[cfg(feature = "ssz")]
mod ssz_requests_conversions {
    use super::*;
    use alloy_eips::{
        eip6110::{DepositRequest, DEPOSIT_REQUEST_TYPE, MAX_DEPOSIT_RECEIPTS_PER_PAYLOAD},
        eip7002::{WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_BLOCK, WITHDRAWAL_REQUEST_TYPE},
        eip7251::{
            ConsolidationRequest, CONSOLIDATION_REQUEST_TYPE, MAX_CONSOLIDATION_REQUESTS_PER_BLOCK,
        },
        eip7685::Requests,
    };
    use ssz::{Decode, DecodeError, Encode};

    impl TryFrom<&Requests> for ExecutionRequestsV4 {
        type Error = TryFromRequestsError;
        fn try_from(value: &Requests) -> Result<Self, Self::Error> {
            let (deposits, withdrawals, consolidations) = value.iter().try_fold(
                (Vec::new(), Vec::new(), Vec::new()),
                |mut acc, request| {
                    if request.is_empty() {
                        return Err(TryFromRequestsError::EmptyRequest);
                    }

                    match request[0] {
                        DEPOSIT_REQUEST_TYPE => {
                            let list: Vec<DepositRequest> = Vec::from_ssz_bytes(&request[1..])?;
                            let size = list.len();
                            if size > MAX_DEPOSIT_RECEIPTS_PER_PAYLOAD {
                                return Err(TryFromRequestsError::RequestPayloadSizeExceeded(
                                    DEPOSIT_REQUEST_TYPE,
                                    size,
                                ));
                            }
                            acc.0.extend(list);
                        }
                        WITHDRAWAL_REQUEST_TYPE => {
                            let list: Vec<WithdrawalRequest> = Vec::from_ssz_bytes(&request[1..])?;
                            let size = list.len();
                            if size > MAX_WITHDRAWAL_REQUESTS_PER_BLOCK {
                                return Err(TryFromRequestsError::RequestPayloadSizeExceeded(
                                    WITHDRAWAL_REQUEST_TYPE,
                                    size,
                                ));
                            }
                            acc.1.extend(list);
                        }
                        CONSOLIDATION_REQUEST_TYPE => {
                            let list: Vec<ConsolidationRequest> =
                                Vec::from_ssz_bytes(&request[1..])?;
                            let size = list.len();
                            if size > MAX_CONSOLIDATION_REQUESTS_PER_BLOCK {
                                return Err(TryFromRequestsError::RequestPayloadSizeExceeded(
                                    CONSOLIDATION_REQUEST_TYPE,
                                    size,
                                ));
                            }
                            acc.2.extend(list);
                        }
                        unknown => return Err(TryFromRequestsError::UnknownRequestType(unknown)),
                    }
                    Ok(acc)
                },
            )?;

            Ok(Self { deposits, withdrawals, consolidations })
        }
    }

    /// Errors possible converting a [Requests] to [ExecutionRequestsV4]
    #[derive(Debug)]
    pub enum TryFromRequestsError {
        /// One of the Bytes is empty.
        EmptyRequest,
        /// Bytes prefix is not a known EIP-7685 request_type in Electra.
        UnknownRequestType(u8),
        /// Remaining bytes could not be decoded as SSZ requests_data.
        SszDecodeError(DecodeError),
        /// Requests of request_type exceeds Electra size limits
        RequestPayloadSizeExceeded(u8, usize),
    }

    impl From<DecodeError> for TryFromRequestsError {
        fn from(value: DecodeError) -> Self {
            Self::SszDecodeError(value)
        }
    }

    impl From<&ExecutionRequestsV4> for Requests {
        fn from(val: &ExecutionRequestsV4) -> Self {
            let deposit_bytes = val.deposits.as_ssz_bytes();
            let withdrawals_bytes = val.withdrawals.as_ssz_bytes();
            let consolidations_bytes = val.consolidations.as_ssz_bytes();

            let mut requests = Self::default();
            requests.push_request_with_type(DEPOSIT_REQUEST_TYPE, deposit_bytes);
            requests.push_request_with_type(WITHDRAWAL_REQUEST_TYPE, withdrawals_bytes);
            requests.push_request_with_type(CONSOLIDATION_REQUEST_TYPE, consolidations_bytes);
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
    }
}
