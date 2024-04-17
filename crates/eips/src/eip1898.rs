//! [EIP-1898]: https://eips.ethereum.org/EIPS/eip-1898

use alloy_primitives::{hex::FromHexError, ruint::ParseError, B256, U64};
use core::{
    fmt::{self, Debug, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

#[cfg(feature = "serde")]
use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

/// A block hash which may have
/// a boolean requireCanonical field.
/// If false, an RPC call should raise if a block
/// matching the hash is not found.
/// If true, an RPC call should additionally raise if
/// the block is not in the canonical chain.
/// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md#specification>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename = "camelCase"))]
pub struct RpcBlockHash {
    /// A block hash
    pub block_hash: B256,
    /// Whether the block must be a canonical block
    pub require_canonical: Option<bool>,
}

impl RpcBlockHash {
    /// Returns an [RpcBlockHash] from a [B256].
    pub const fn from_hash(block_hash: B256, require_canonical: Option<bool>) -> Self {
        RpcBlockHash { block_hash, require_canonical }
    }
}

impl From<B256> for RpcBlockHash {
    fn from(value: B256) -> Self {
        Self::from_hash(value, None)
    }
}

impl From<RpcBlockHash> for B256 {
    fn from(value: RpcBlockHash) -> Self {
        value.block_hash
    }
}

impl AsRef<B256> for RpcBlockHash {
    fn as_ref(&self) -> &B256 {
        &self.block_hash
    }
}

/// A block Number (or tag - "latest", "earliest", "pending")
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlockNumberOrTag {
    /// Latest block
    #[default]
    Latest,
    /// Finalized block accepted as canonical
    Finalized,
    /// Safe head block
    Safe,
    /// Earliest block (genesis)
    Earliest,
    /// Pending block (not yet part of the blockchain)
    Pending,
    /// Block by number from canon chain
    Number(u64),
}

impl BlockNumberOrTag {
    /// Returns the numeric block number if explicitly set
    pub const fn as_number(&self) -> Option<u64> {
        match *self {
            BlockNumberOrTag::Number(num) => Some(num),
            _ => None,
        }
    }

    /// Returns `true` if a numeric block number is set
    pub const fn is_number(&self) -> bool {
        matches!(self, BlockNumberOrTag::Number(_))
    }

    /// Returns `true` if it's "latest"
    pub const fn is_latest(&self) -> bool {
        matches!(self, BlockNumberOrTag::Latest)
    }

    /// Returns `true` if it's "finalized"
    pub const fn is_finalized(&self) -> bool {
        matches!(self, BlockNumberOrTag::Finalized)
    }

    /// Returns `true` if it's "safe"
    pub const fn is_safe(&self) -> bool {
        matches!(self, BlockNumberOrTag::Safe)
    }

    /// Returns `true` if it's "pending"
    pub const fn is_pending(&self) -> bool {
        matches!(self, BlockNumberOrTag::Pending)
    }

    /// Returns `true` if it's "earliest"
    pub const fn is_earliest(&self) -> bool {
        matches!(self, BlockNumberOrTag::Earliest)
    }
}

impl From<u64> for BlockNumberOrTag {
    fn from(num: u64) -> Self {
        BlockNumberOrTag::Number(num)
    }
}

impl From<U64> for BlockNumberOrTag {
    fn from(num: U64) -> Self {
        num.to::<u64>().into()
    }
}

#[cfg(feature = "serde")]
impl Serialize for BlockNumberOrTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            BlockNumberOrTag::Number(x) => serializer.serialize_str(&format!("0x{x:x}")),
            BlockNumberOrTag::Latest => serializer.serialize_str("latest"),
            BlockNumberOrTag::Finalized => serializer.serialize_str("finalized"),
            BlockNumberOrTag::Safe => serializer.serialize_str("safe"),
            BlockNumberOrTag::Earliest => serializer.serialize_str("earliest"),
            BlockNumberOrTag::Pending => serializer.serialize_str("pending"),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for BlockNumberOrTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = alloc::string::String::deserialize(deserializer)?.to_lowercase();
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for BlockNumberOrTag {
    type Err = ParseBlockNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let block = match s {
            "latest" => Self::Latest,
            "finalized" => Self::Finalized,
            "safe" => Self::Safe,
            "earliest" => Self::Earliest,
            "pending" => Self::Pending,
            _number => {
                if let Some(hex_val) = s.strip_prefix("0x") {
                    let number = u64::from_str_radix(hex_val, 16);
                    BlockNumberOrTag::Number(number?)
                } else {
                    return Err(HexStringMissingPrefixError::default().into());
                }
            }
        };
        Ok(block)
    }
}

impl fmt::Display for BlockNumberOrTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockNumberOrTag::Number(x) => write!(f, "0x{x:x}"),
            BlockNumberOrTag::Latest => f.write_str("latest"),
            BlockNumberOrTag::Finalized => f.write_str("finalized"),
            BlockNumberOrTag::Safe => f.write_str("safe"),
            BlockNumberOrTag::Earliest => f.write_str("earliest"),
            BlockNumberOrTag::Pending => f.write_str("pending"),
        }
    }
}

/// Error thrown when parsing a [BlockNumberOrTag] from a string.
#[derive(Debug)]
pub enum ParseBlockNumberError {
    /// Failed to parse hex value
    ParseIntErr(ParseIntError),
    /// Failed to parse hex value
    ParseErr(ParseError),
    /// Block numbers should be 0x-prefixed
    MissingPrefix(HexStringMissingPrefixError),
}

/// Error variants when parsing a [BlockNumberOrTag]
#[cfg(feature = "std")]
impl std::error::Error for ParseBlockNumberError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseBlockNumberError::ParseIntErr(err) => std::error::Error::source(err),
            ParseBlockNumberError::ParseErr(err) => std::error::Error::source(err),
            ParseBlockNumberError::MissingPrefix(err) => std::error::Error::source(err),
        }
    }
}

impl Display for ParseBlockNumberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseIntErr(err) => write!(f, "{err}"),
            Self::ParseErr(err) => write!(f, "{err}"),
            Self::MissingPrefix(err) => write!(f, "{err}"),
        }
    }
}

impl From<ParseIntError> for ParseBlockNumberError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseIntErr(err)
    }
}

impl From<ParseError> for ParseBlockNumberError {
    fn from(err: ParseError) -> Self {
        Self::ParseErr(err)
    }
}

impl From<HexStringMissingPrefixError> for ParseBlockNumberError {
    fn from(err: HexStringMissingPrefixError) -> Self {
        Self::MissingPrefix(err)
    }
}

/// Thrown when a 0x-prefixed hex string was expected
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct HexStringMissingPrefixError;

impl Display for HexStringMissingPrefixError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("hex string without 0x prefix")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HexStringMissingPrefixError {}

/// A Block Identifier.
/// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockId {
    /// A block hash and an optional bool that defines if it's canonical
    Hash(RpcBlockHash),
    /// A block number
    Number(BlockNumberOrTag),
}

// === impl BlockId ===

impl BlockId {
    /// Returns the block hash if it is [BlockId::Hash]
    pub const fn as_block_hash(&self) -> Option<B256> {
        match self {
            BlockId::Hash(hash) => Some(hash.block_hash),
            BlockId::Number(_) => None,
        }
    }

    /// Returns true if this is [BlockNumberOrTag::Latest]
    pub const fn is_latest(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Latest))
    }

    /// Returns true if this is [BlockNumberOrTag::Pending]
    pub const fn is_pending(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Pending))
    }

    /// Returns true if this is [BlockNumberOrTag::Safe]
    pub const fn is_safe(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Safe))
    }

    /// Returns true if this is [BlockNumberOrTag::Finalized]
    pub const fn is_finalized(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Finalized))
    }

    /// Returns true if this is [BlockNumberOrTag::Earliest]
    pub const fn is_earliest(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Earliest))
    }

    /// Returns true if this is [BlockNumberOrTag::Number]
    pub const fn is_number(&self) -> bool {
        matches!(self, BlockId::Number(BlockNumberOrTag::Number(_)))
    }
    /// Returns true if this is [BlockId::Hash]
    pub const fn is_hash(&self) -> bool {
        matches!(self, BlockId::Hash(_))
    }

    /// Creates a new "pending" tag instance.
    pub const fn pending() -> Self {
        BlockId::Number(BlockNumberOrTag::Pending)
    }

    /// Creates a new "latest" tag instance.
    pub const fn latest() -> Self {
        BlockId::Number(BlockNumberOrTag::Latest)
    }

    /// Creates a new "earliest" tag instance.
    pub const fn earliest() -> Self {
        BlockId::Number(BlockNumberOrTag::Earliest)
    }

    /// Creates a new "finalized" tag instance.
    pub const fn finalized() -> Self {
        BlockId::Number(BlockNumberOrTag::Finalized)
    }

    /// Creates a new "safe" tag instance.
    pub const fn safe() -> Self {
        BlockId::Number(BlockNumberOrTag::Safe)
    }

    /// Creates a new block number instance.
    pub const fn number(num: u64) -> Self {
        BlockId::Number(BlockNumberOrTag::Number(num))
    }

    /// Create a new block hash instance.
    pub const fn hash(block_hash: B256) -> Self {
        BlockId::Hash(RpcBlockHash { block_hash, require_canonical: None })
    }

    /// Create a new block hash instance that requires the block to be canonical.
    pub const fn hash_canonical(block_hash: B256) -> Self {
        BlockId::Hash(RpcBlockHash { block_hash, require_canonical: Some(true) })
    }
}

impl Default for BlockId {
    fn default() -> Self {
        BlockId::Number(BlockNumberOrTag::Latest)
    }
}

impl From<u64> for BlockId {
    fn from(num: u64) -> Self {
        BlockNumberOrTag::Number(num).into()
    }
}

impl From<U64> for BlockId {
    fn from(value: U64) -> Self {
        BlockNumberOrTag::Number(value.to()).into()
    }
}

impl From<BlockNumberOrTag> for BlockId {
    fn from(num: BlockNumberOrTag) -> Self {
        BlockId::Number(num)
    }
}

impl From<B256> for BlockId {
    fn from(block_hash: B256) -> Self {
        BlockId::Hash(RpcBlockHash { block_hash, require_canonical: None })
    }
}

impl From<(B256, Option<bool>)> for BlockId {
    fn from(hash_can: (B256, Option<bool>)) -> Self {
        BlockId::Hash(RpcBlockHash { block_hash: hash_can.0, require_canonical: hash_can.1 })
    }
}

#[cfg(feature = "serde")]
impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BlockId::Hash(RpcBlockHash { block_hash, require_canonical }) => {
                let mut s = serializer.serialize_struct("BlockIdEip1898", 1)?;
                s.serialize_field("blockHash", block_hash)?;
                if let Some(require_canonical) = require_canonical {
                    s.serialize_field("requireCanonical", require_canonical)?;
                }
                s.end()
            }
            BlockId::Number(num) => num.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BlockIdVisitor;

        impl<'de> Visitor<'de> for BlockIdVisitor {
            type Value = BlockId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("Block identifier following EIP-1898")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Since there is no way to clearly distinguish between a DATA parameter and a QUANTITY parameter. A str is therefor deserialized into a Block Number: <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
                // However, since the hex string should be a QUANTITY, we can safely assume that if the len is 66 bytes, it is in fact a hash, ref <https://github.com/ethereum/go-ethereum/blob/ee530c0d5aa70d2c00ab5691a89ab431b73f8165/rpc/types.go#L184-L184>
                if v.len() == 66 {
                    Ok(BlockId::Hash(v.parse::<B256>().map_err(serde::de::Error::custom)?.into()))
                } else {
                    // quantity hex string or tag
                    Ok(BlockId::Number(v.parse().map_err(serde::de::Error::custom)?))
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut number = None;
                let mut block_hash = None;
                let mut require_canonical = None;
                while let Some(key) = map.next_key::<alloc::string::String>()? {
                    match key.as_str() {
                        "blockNumber" => {
                            if number.is_some() || block_hash.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockNumber"));
                            }
                            if require_canonical.is_some() {
                                return Err(serde::de::Error::custom(
                                    "Non-valid require_canonical field",
                                ));
                            }
                            number = Some(map.next_value::<BlockNumberOrTag>()?)
                        }
                        "blockHash" => {
                            if number.is_some() || block_hash.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }

                            block_hash = Some(map.next_value::<B256>()?);
                        }
                        "requireCanonical" => {
                            if number.is_some() || require_canonical.is_some() {
                                return Err(serde::de::Error::duplicate_field("requireCanonical"));
                            }

                            require_canonical = Some(map.next_value::<bool>()?)
                        }
                        key => {
                            return Err(serde::de::Error::unknown_field(
                                key,
                                &["blockNumber", "blockHash", "requireCanonical"],
                            ))
                        }
                    }
                }

                if let Some(number) = number {
                    Ok(BlockId::Number(number))
                } else if let Some(block_hash) = block_hash {
                    Ok(BlockId::Hash(RpcBlockHash { block_hash, require_canonical }))
                } else {
                    Err(serde::de::Error::custom(
                        "Expected `blockNumber` or `blockHash` with `requireCanonical` optionally",
                    ))
                }
            }
        }

        deserializer.deserialize_any(BlockIdVisitor)
    }
}

/// Error thrown when parsing a [BlockId] from a string.
#[derive(Debug)]
pub enum ParseBlockIdError {
    /// Failed to parse a block id from a number.
    ParseIntError(ParseIntError),
    /// Failed to parse a block id as a hex string.
    FromHexError(FromHexError),
}

impl Display for ParseBlockIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseIntError(err) => write!(f, "{err}"),
            Self::FromHexError(err) => write!(f, "{err}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseBlockIdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseBlockIdError::ParseIntError(err) => std::error::Error::source(err),
            ParseBlockIdError::FromHexError(err) => std::error::Error::source(err),
        }
    }
}

impl From<ParseIntError> for ParseBlockIdError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseIntError(err)
    }
}

impl From<FromHexError> for ParseBlockIdError {
    fn from(err: FromHexError) -> Self {
        Self::FromHexError(err)
    }
}

impl FromStr for BlockId {
    type Err = ParseBlockIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("0x") {
            return B256::from_str(s).map(Into::into).map_err(ParseBlockIdError::FromHexError);
        }

        match s {
            "latest" => Ok(BlockId::Number(BlockNumberOrTag::Latest)),
            "finalized" => Ok(BlockId::Number(BlockNumberOrTag::Finalized)),
            "safe" => Ok(BlockId::Number(BlockNumberOrTag::Safe)),
            "earliest" => Ok(BlockId::Number(BlockNumberOrTag::Earliest)),
            "pending" => Ok(BlockId::Number(BlockNumberOrTag::Pending)),
            _ => s
                .parse::<u64>()
                .map_err(ParseBlockIdError::ParseIntError)
                .map(|n| BlockId::Number(n.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_block_number_serde() {
        let num: BlockNumberOrTag = 1u64.into();
        let serialized = serde_json::to_string(&num).unwrap();
        assert_eq!(serialized, "\"0x1\"");
    }

    #[test]
    fn can_parse_eip1898_block_ids() {
        let num = serde_json::json!(
            { "blockNumber": "0x0" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Number(0u64)));

        let num = serde_json::json!(
            { "blockNumber": "pending" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Pending));

        let num = serde_json::json!(
            { "blockNumber": "latest" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Latest));

        let num = serde_json::json!(
            { "blockNumber": "finalized" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Finalized));

        let num = serde_json::json!(
            { "blockNumber": "safe" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Safe));

        let num = serde_json::json!(
            { "blockNumber": "earliest" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Earliest));

        let num = serde_json::json!("0x0");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Number(0u64)));

        let num = serde_json::json!("pending");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Pending));

        let num = serde_json::json!("latest");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Latest));

        let num = serde_json::json!("finalized");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Finalized));

        let num = serde_json::json!("safe");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Safe));

        let num = serde_json::json!("earliest");
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(id, BlockId::Number(BlockNumberOrTag::Earliest));

        let num = serde_json::json!(
            { "blockHash": "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3" }
        );
        let id = serde_json::from_value::<BlockId>(num).unwrap();
        assert_eq!(
            id,
            BlockId::Hash(
                "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3"
                    .parse::<B256>()
                    .unwrap()
                    .into()
            )
        );
    }
}
