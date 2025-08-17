use alloy_consensus::{error::ValueError, BlockHeader, Header};
use alloy_primitives::{Address, BlockNumber, Bloom, Bytes, Sealed, B256, B64, U256};

/// Block header representation with certain fields made optional to account for possible
/// differences in network implementations.
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AnyHeader {
    /// Hash of the parent
    pub parent_hash: B256,
    /// Hash of the uncles
    #[cfg_attr(feature = "serde", serde(rename = "sha3Uncles"))]
    pub ommers_hash: B256,
    /// Alias of `author`
    #[cfg_attr(feature = "serde", serde(rename = "miner"))]
    pub beneficiary: Address,
    /// State root hash
    #[cfg_attr(feature = "serde", serde(deserialize_with = "lenient_state_root"))]
    pub state_root: B256,
    /// Transactions root hash
    pub transactions_root: B256,
    /// Transactions receipts root hash
    pub receipts_root: B256,
    /// Logs bloom
    pub logs_bloom: Bloom,
    /// Difficulty
    pub difficulty: U256,
    /// Block number
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub number: u64,
    /// Gas Limit
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub gas_limit: u64,
    /// Gas Used
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub gas_used: u64,
    /// Timestamp
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity"))]
    pub timestamp: u64,
    /// Extra data
    pub extra_data: Bytes,
    /// Mix Hash
    ///
    /// Before the merge this proves, combined with the nonce, that a sufficient amount of
    /// computation has been carried out on this block: the Proof-of-Work (PoW).
    ///
    /// After the merge this is `prevRandao`: Randomness value for the generated payload.
    ///
    /// This is an Option because it is not always set by non-ethereum networks.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-4399>
    /// And <https://github.com/ethereum/execution-apis/issues/328>
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub mix_hash: Option<B256>,
    /// Nonce
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub nonce: Option<B64>,
    /// Base fee per unit of gas (if past London)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub base_fee_per_gas: Option<u64>,
    /// Withdrawals root hash added by EIP-4895 and is ignored in legacy headers.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub withdrawals_root: Option<B256>,
    /// Blob gas used
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub blob_gas_used: Option<u64>,
    /// Excess blob gas
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub excess_blob_gas: Option<u64>,
    /// EIP-4788 parent beacon block root
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub parent_beacon_block_root: Option<B256>,
    /// EIP-7685 requests hash.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub requests_hash: Option<B256>,
    /// EIP-7928 block access list hash.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub block_access_list_hash: Option<B256>,
}

impl AnyHeader {
    /// Seal the header with a known hash.
    ///
    /// WARNING: This method does not perform validation whether the hash is correct.
    #[inline]
    pub const fn seal(self, hash: B256) -> Sealed<Self> {
        Sealed::new_unchecked(self, hash)
    }

    /// Attempts to convert this header into a `Header`.
    ///
    /// This can fail if the header is missing required fields:
    /// - nonce
    /// - mix_hash
    ///
    /// If the conversion fails, the original [`AnyHeader`] is returned.
    pub fn try_into_header(self) -> Result<Header, ValueError<Self>> {
        if self.nonce.is_none() {
            return Err(ValueError::new(self, "missing nonce field"));
        }
        if self.mix_hash.is_none() {
            return Err(ValueError::new(self, "missing mix hash field"));
        }

        let Self {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            nonce,
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        } = self;

        Ok(Header {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash: mix_hash.unwrap(),
            nonce: nonce.unwrap(),
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        })
    }

    /// Converts this header into a [`Header`] with default values for missing mandatory fields:
    /// - mix_hash
    /// - nonce
    pub fn into_header_with_defaults(self) -> Header {
        let Self {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            nonce,
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        } = self;

        Header {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash: mix_hash.unwrap_or_default(),
            nonce: nonce.unwrap_or_default(),
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        }
    }
}

impl BlockHeader for AnyHeader {
    fn parent_hash(&self) -> B256 {
        self.parent_hash
    }

    fn ommers_hash(&self) -> B256 {
        self.ommers_hash
    }

    fn beneficiary(&self) -> Address {
        self.beneficiary
    }

    fn state_root(&self) -> B256 {
        self.state_root
    }

    fn transactions_root(&self) -> B256 {
        self.transactions_root
    }

    fn receipts_root(&self) -> B256 {
        self.receipts_root
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.withdrawals_root
    }

    fn logs_bloom(&self) -> Bloom {
        self.logs_bloom
    }

    fn difficulty(&self) -> U256 {
        self.difficulty
    }

    fn number(&self) -> BlockNumber {
        self.number
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_used(&self) -> u64 {
        self.gas_used
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn mix_hash(&self) -> Option<B256> {
        self.mix_hash
    }

    fn nonce(&self) -> Option<B64> {
        self.nonce
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.blob_gas_used
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.excess_blob_gas
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.parent_beacon_block_root
    }

    fn requests_hash(&self) -> Option<B256> {
        self.requests_hash
    }

    fn extra_data(&self) -> &Bytes {
        &self.extra_data
    }
}

impl From<Header> for AnyHeader {
    fn from(value: Header) -> Self {
        let Header {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            nonce,
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        } = value;

        Self {
            parent_hash,
            ommers_hash,
            beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash: Some(mix_hash),
            nonce: Some(nonce),
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
        }
    }
}

impl TryFrom<AnyHeader> for Header {
    type Error = ValueError<AnyHeader>;

    fn try_from(value: AnyHeader) -> Result<Self, Self::Error> {
        value.try_into_header()
    }
}

/// Custom deserializer for `state_root` that treats `"0x"` or empty as `B256::ZERO`
///
/// This exists because some networks (like Tron) may serialize the state root as `"0x"`
#[cfg(feature = "serde")]
fn lenient_state_root<'de, D>(deserializer: D) -> Result<B256, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use alloc::string::String;
    use core::str::FromStr;
    use serde::de::Error;

    let s: String = serde::de::Deserialize::deserialize(deserializer)?;
    let s = s.trim();

    if s == "0x" || s.is_empty() {
        return Ok(B256::ZERO);
    }

    B256::from_str(s).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {

    // <https://github.com/alloy-rs/alloy/issues/2494>
    #[test]
    #[cfg(feature = "serde")]
    fn deserializes_tron_state_root_in_header() {
        use super::*;
        use alloy_primitives::B256;

        let s = r#"{
  "baseFeePerGas": "0x0",
  "difficulty": "0x0",
  "extraData": "0x",
  "gasLimit": "0x160227b88",
  "gasUsed": "0x360d92",
  "hash": "0x00000000040a0687e0fc7194aabd024a4786ce94ad63855774f8d48896d8750b",
  "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "miner": "0x9a96c8003a1e3a6866c08acff9f629e2a6ef062b",
  "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "nonce": "0x0000000000000000",
  "number": "0x40a0687",
  "parentHash": "0x00000000040a068652c581a982a0d17976201ad44aa28eb4e24881e82f99ee04",
  "receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "sha3Uncles": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "transactionsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "size": "0xba05",
  "stateRoot": "0x",
  "timestamp": "0x6759f2f1",
  "totalDifficulty": "0x0"
}"#;

        let header: AnyHeader = serde_json::from_str(s).unwrap();
        assert_eq!(header.state_root, B256::ZERO);
    }
}
