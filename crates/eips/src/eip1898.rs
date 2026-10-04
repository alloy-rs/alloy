//! [EIP-1898]: https://eips.ethereum.org/EIPS/eip-1898

use alloy_primitives::{hex::FromHexError, ruint::ParseError, BlockHash, B256, U64};
use alloy_rlp::{bytes, Decodable, Encodable, Error as RlpError};
use core::{
    fmt::{self, Formatter},
    num::ParseIntError,
    str::FromStr,
};

/// A helper struct to store the block number/hash and its parent hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BlockWithParent {
    /// Parent hash.
    pub parent: B256,
    /// Block number/hash.
    pub block: BlockNumHash,
}

impl BlockWithParent {
    /// Creates a new [`BlockWithParent`] instance.
    pub const fn new(parent: B256, block: BlockNumHash) -> Self {
        Self { parent, block }
    }
}

/// A block hash which may have a boolean `requireCanonical` field.
///
/// - If false, a RPC call should raise if a block matching the hash is not found.
/// - If true, a RPC call should additionally raise if the block is not in the canonical chain.
///
/// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md#specification>
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RpcBlockHash {
    /// A block hash
    pub block_hash: BlockHash,
    /// Whether the block must be a canonical block
    pub require_canonical: Option<bool>,
}

impl RpcBlockHash {
    /// Returns a [`RpcBlockHash`] from a [`B256`].
    #[doc(alias = "from_block_hash")]
    pub const fn from_hash(block_hash: B256, require_canonical: Option<bool>) -> Self {
        Self { block_hash, require_canonical }
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

impl fmt::Display for RpcBlockHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { block_hash, require_canonical } = self;
        if *require_canonical == Some(true) {
            f.write_str("canonical ")?;
        }
        write!(f, "hash {block_hash}")
    }
}

impl fmt::Debug for RpcBlockHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.require_canonical {
            Some(require_canonical) => f
                .debug_struct("RpcBlockHash")
                .field("block_hash", &self.block_hash)
                .field("require_canonical", &require_canonical)
                .finish(),
            None => fmt::Debug::fmt(&self.block_hash, f),
        }
    }
}

/// A block Number (or tag - "latest", "earliest", "pending")
///
/// This enum allows users to specify a block in a flexible manner.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
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
    /// Block by number from canonical chain
    Number(u64),
}

impl BlockNumberOrTag {
    /// Returns the numeric block number if explicitly set
    pub const fn as_number(&self) -> Option<u64> {
        match *self {
            Self::Number(num) => Some(num),
            _ => None,
        }
    }

    /// Returns `true` if a numeric block number is set
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    /// Returns `true` if it's "latest"
    pub const fn is_latest(&self) -> bool {
        matches!(self, Self::Latest)
    }

    /// Returns `true` if it's "finalized"
    pub const fn is_finalized(&self) -> bool {
        matches!(self, Self::Finalized)
    }

    /// Returns `true` if it's "safe"
    pub const fn is_safe(&self) -> bool {
        matches!(self, Self::Safe)
    }

    /// Returns `true` if it's "pending"
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if it's "earliest"
    pub const fn is_earliest(&self) -> bool {
        matches!(self, Self::Earliest)
    }
}

impl From<u64> for BlockNumberOrTag {
    fn from(num: u64) -> Self {
        Self::Number(num)
    }
}

impl From<U64> for BlockNumberOrTag {
    fn from(num: U64) -> Self {
        num.to::<u64>().into()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for BlockNumberOrTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Self::Number(x) => alloy_serde::quantity::serialize(&x, serializer),
            Self::Latest => serializer.serialize_str("latest"),
            Self::Finalized => serializer.serialize_str("finalized"),
            Self::Safe => serializer.serialize_str("safe"),
            Self::Earliest => serializer.serialize_str("earliest"),
            Self::Pending => serializer.serialize_str("pending"),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BlockNumberOrTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = alloc::string::String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for BlockNumberOrTag {
    type Err = ParseBlockNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "latest" => Self::Latest,
            "finalized" => Self::Finalized,
            "safe" => Self::Safe,
            "earliest" => Self::Earliest,
            "pending" => Self::Pending,
            s => {
                if let Some(hex_val) = s.strip_prefix("0x") {
                    u64::from_str_radix(hex_val, 16)?.into()
                } else {
                    return Err(HexStringMissingPrefixError::default().into());
                }
            }
        })
    }
}

impl fmt::Display for BlockNumberOrTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Number(x) => write!(f, "0x{x:x}"),
            Self::Latest => f.pad("latest"),
            Self::Finalized => f.pad("finalized"),
            Self::Safe => f.pad("safe"),
            Self::Earliest => f.pad("earliest"),
            Self::Pending => f.pad("pending"),
        }
    }
}

impl fmt::Debug for BlockNumberOrTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// This is a helper type to allow for lenient parsing of block numbers.
///
/// The eth json-rpc spec requires quantities to be hex encoded, which [`BlockNumberOrTag`] strictly
/// enforces.
///
/// This type can be used if you want to allow for lenient parsing of block numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct LenientBlockNumberOrTag(BlockNumberOrTag);

impl LenientBlockNumberOrTag {
    /// Creates a new [`LenientBlockNumberOrTag`] from a [`BlockNumberOrTag`].
    pub const fn new(block_number_or_tag: BlockNumberOrTag) -> Self {
        Self(block_number_or_tag)
    }

    /// Returns the inner [`BlockNumberOrTag`].
    pub const fn into_inner(self) -> BlockNumberOrTag {
        self.0
    }
}

impl From<LenientBlockNumberOrTag> for BlockNumberOrTag {
    fn from(value: LenientBlockNumberOrTag) -> Self {
        value.0
    }
}
impl From<BlockNumberOrTag> for LenientBlockNumberOrTag {
    fn from(value: BlockNumberOrTag) -> Self {
        Self(value)
    }
}

impl From<LenientBlockNumberOrTag> for BlockId {
    fn from(value: LenientBlockNumberOrTag) -> Self {
        value.into_inner().into()
    }
}

/// A module that deserializes either a BlockNumberOrTag, or a simple number.
#[cfg(feature = "serde")]
pub mod lenient_block_number_or_tag {
    use super::{BlockNumberOrTag, LenientBlockNumberOrTag};
    use core::fmt;
    use serde::{
        de::{self, Visitor},
        Deserialize, Deserializer,
    };

    /// Following the spec the block parameter is either:
    ///
    /// > HEX String - an integer block number
    /// > Integer - a block number
    /// > String "latest" - for the latest mined block
    /// > String "finalized" - for the finalized block
    /// > String "safe" - for the safe head block
    /// > String "earliest" for the earliest/genesis block
    /// > String "pending" - for the pending state/transactions
    ///
    /// and with EIP-1898:
    /// > blockNumber: QUANTITY - a block number
    /// > blockHash: DATA - a block hash
    ///
    /// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
    ///
    /// EIP-1898 does not all calls that use `BlockNumber` like `eth_getBlockByNumber` and doesn't
    /// list raw integers as supported.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<BlockNumberOrTag, D::Error>
    where
        D: Deserializer<'de>,
    {
        LenientBlockNumberOrTag::deserialize(deserializer).map(Into::into)
    }

    /// Serde functions for leniently deserializing an optional [`BlockNumberOrTag`].
    pub mod opt {
        use super::{BlockNumberOrTag, LenientBlockNumberOrTag};
        use serde::{Deserialize, Deserializer};

        /// Deserializes `null` as [`None`] and any value accepted by
        /// [`deserialize`](super::deserialize) as [`Some`].
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<BlockNumberOrTag>, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Option::<LenientBlockNumberOrTag>::deserialize(deserializer)?.map(Into::into))
        }
    }

    impl<'de> Deserialize<'de> for LenientBlockNumberOrTag {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct LenientBlockNumberVisitor;

            impl<'de> Visitor<'de> for LenientBlockNumberVisitor {
                type Value = LenientBlockNumberOrTag;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a block number or tag, or a u64")
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(LenientBlockNumberOrTag(v.into()))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    // Attempt to parse as a numeric string
                    if let Ok(num) = v.parse::<u64>() {
                        return Ok(LenientBlockNumberOrTag(BlockNumberOrTag::Number(num)));
                    }

                    BlockNumberOrTag::deserialize(de::value::StrDeserializer::<E>::new(v))
                        .map(LenientBlockNumberOrTag)
                        .map_err(de::Error::custom)
                }
            }

            deserializer.deserialize_any(LenientBlockNumberVisitor)
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
impl core::error::Error for ParseBlockNumberError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ParseIntErr(err) => Some(err),
            Self::MissingPrefix(err) => Some(err),
            Self::ParseErr(_) => None,
        }
    }
}

impl fmt::Display for ParseBlockNumberError {
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

impl fmt::Display for HexStringMissingPrefixError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("hex string without 0x prefix")
    }
}

impl core::error::Error for HexStringMissingPrefixError {}

/// A Block Identifier.
/// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockId {
    /// A block hash and an optional bool that defines if it's canonical
    Hash(RpcBlockHash),
    /// A block number
    Number(BlockNumberOrTag),
}

impl BlockId {
    /// Returns the block hash if it is [BlockId::Hash]
    pub const fn as_block_hash(&self) -> Option<BlockHash> {
        match self {
            Self::Hash(hash) => Some(hash.block_hash),
            Self::Number(_) => None,
        }
    }

    /// Returns the block number if it is [`BlockId::Number`] and not a tag
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(x) => x.as_number(),
            _ => None,
        }
    }

    /// Returns true if this is [BlockNumberOrTag::Latest]
    pub const fn is_latest(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Latest))
    }

    /// Returns true if this is [BlockNumberOrTag::Pending]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Pending))
    }

    /// Returns true if this is [BlockNumberOrTag::Safe]
    pub const fn is_safe(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Safe))
    }

    /// Returns true if this is [BlockNumberOrTag::Finalized]
    pub const fn is_finalized(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Finalized))
    }

    /// Returns true if this is [BlockNumberOrTag::Earliest]
    pub const fn is_earliest(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Earliest))
    }

    /// Returns true if this is [BlockNumberOrTag::Number]
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(BlockNumberOrTag::Number(_)))
    }
    /// Returns true if this is [BlockId::Hash]
    pub const fn is_hash(&self) -> bool {
        matches!(self, Self::Hash(_))
    }

    /// Creates a new "pending" tag instance.
    pub const fn pending() -> Self {
        Self::Number(BlockNumberOrTag::Pending)
    }

    /// Creates a new "latest" tag instance.
    pub const fn latest() -> Self {
        Self::Number(BlockNumberOrTag::Latest)
    }

    /// Creates a new "earliest" tag instance.
    pub const fn earliest() -> Self {
        Self::Number(BlockNumberOrTag::Earliest)
    }

    /// Creates a new "finalized" tag instance.
    pub const fn finalized() -> Self {
        Self::Number(BlockNumberOrTag::Finalized)
    }

    /// Creates a new "safe" tag instance.
    pub const fn safe() -> Self {
        Self::Number(BlockNumberOrTag::Safe)
    }

    /// Creates a new block number instance.
    pub const fn number(num: u64) -> Self {
        Self::Number(BlockNumberOrTag::Number(num))
    }

    /// Create a new block hash instance.
    pub const fn hash(block_hash: BlockHash) -> Self {
        Self::Hash(RpcBlockHash { block_hash, require_canonical: None })
    }

    /// Create a new block hash instance that requires the block to be canonical.
    pub const fn hash_canonical(block_hash: BlockHash) -> Self {
        Self::Hash(RpcBlockHash { block_hash, require_canonical: Some(true) })
    }
}

impl Default for BlockId {
    fn default() -> Self {
        BlockNumberOrTag::Latest.into()
    }
}

impl From<u64> for BlockId {
    fn from(num: u64) -> Self {
        BlockNumberOrTag::Number(num).into()
    }
}

impl From<U64> for BlockId {
    fn from(value: U64) -> Self {
        value.to::<u64>().into()
    }
}

impl From<BlockNumberOrTag> for BlockId {
    fn from(num: BlockNumberOrTag) -> Self {
        Self::Number(num)
    }
}

impl From<HashOrNumber> for BlockId {
    fn from(block: HashOrNumber) -> Self {
        match block {
            HashOrNumber::Hash(hash) => hash.into(),
            HashOrNumber::Number(num) => num.into(),
        }
    }
}

impl From<B256> for BlockId {
    fn from(block_hash: B256) -> Self {
        RpcBlockHash { block_hash, require_canonical: None }.into()
    }
}

impl From<(B256, Option<bool>)> for BlockId {
    fn from(hash_can: (B256, Option<bool>)) -> Self {
        RpcBlockHash { block_hash: hash_can.0, require_canonical: hash_can.1 }.into()
    }
}

impl From<RpcBlockHash> for BlockId {
    fn from(value: RpcBlockHash) -> Self {
        Self::Hash(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Hash(RpcBlockHash { block_hash, require_canonical }) => {
                if let Some(require_canonical) = require_canonical {
                    use serde::ser::SerializeStruct;

                    let mut s = serializer.serialize_struct("BlockIdEip1898", 2)?;
                    s.serialize_field("blockHash", block_hash)?;
                    s.serialize_field("requireCanonical", require_canonical)?;
                    s.end()
                } else {
                    block_hash.serialize(serializer)
                }
            }
            Self::Number(num) => num.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BlockIdVisitor;

        impl<'de> serde::de::Visitor<'de> for BlockIdVisitor {
            type Value = BlockId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("Block identifier following EIP-1898")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Since there is no way to clearly distinguish between a DATA parameter and a QUANTITY parameter. A str is therefore deserialized into a Block Number: <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
                // However, since the hex string should be a QUANTITY, we can safely assume that if the len is 66 bytes, it is in fact a hash, ref <https://github.com/ethereum/go-ethereum/blob/ee530c0d5aa70d2c00ab5691a89ab431b73f8165/rpc/types.go#L184-L184>
                if v.len() == 66 {
                    Ok(v.parse::<B256>().map_err(serde::de::Error::custom)?.into())
                } else {
                    // quantity hex string or tag
                    Ok(BlockId::Number(v.parse().map_err(serde::de::Error::custom)?))
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
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

                #[expect(clippy::option_if_let_else)]
                if let Some(number) = number {
                    Ok(number.into())
                } else if let Some(block_hash) = block_hash {
                    Ok((block_hash, require_canonical).into())
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

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(hash) => hash.fmt(f),
            Self::Number(num) => num.fmt(f),
        }
    }
}

impl fmt::Debug for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(hash) => hash.fmt(f),
            Self::Number(num) => num.fmt(f),
        }
    }
}

/// Error thrown when parsing a [BlockId] from a string.
#[derive(Debug)]
pub enum ParseBlockIdError {
    /// Failed to parse a block id from a number.
    ParseIntError(ParseIntError),
    /// Failed to parse hex number
    ParseError(ParseError),
    /// Failed to parse a block id as a hex string.
    FromHexError(FromHexError),
}

impl fmt::Display for ParseBlockIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseIntError(err) => write!(f, "{err}"),
            Self::ParseError(err) => write!(f, "{err}"),
            Self::FromHexError(err) => write!(f, "{err}"),
        }
    }
}

impl core::error::Error for ParseBlockIdError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ParseIntError(err) => Some(err),
            Self::FromHexError(err) => Some(err),
            Self::ParseError(_) => None,
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
            return match s.len() {
                66 => B256::from_str(s).map(Into::into).map_err(ParseBlockIdError::FromHexError),
                _ => U64::from_str(s).map(Into::into).map_err(ParseBlockIdError::ParseError),
            };
        }

        match s {
            "latest" | "finalized" | "safe" | "earliest" | "pending" => {
                Ok(BlockNumberOrTag::from_str(s).unwrap().into())
            }
            _ => s.parse::<u64>().map_err(ParseBlockIdError::ParseIntError).map(Into::into),
        }
    }
}

/// A number and a hash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct NumHash {
    /// The number
    pub number: u64,
    /// The hash.
    pub hash: B256,
}

/// Block number and hash of the forked block.
pub type ForkBlock = NumHash;

/// A block number and a hash
pub type BlockNumHash = NumHash;

impl NumHash {
    /// Creates a new `NumHash` from a number and hash.
    pub const fn new(number: u64, hash: B256) -> Self {
        Self { number, hash }
    }

    /// Consumes `Self` and returns the number and hash
    pub const fn into_components(self) -> (u64, B256) {
        (self.number, self.hash)
    }

    /// Returns whether or not the block matches the given [HashOrNumber].
    pub fn matches_block_or_num(&self, block: &HashOrNumber) -> bool {
        match block {
            HashOrNumber::Hash(hash) => self.hash == *hash,
            HashOrNumber::Number(number) => self.number == *number,
        }
    }
}

impl From<(u64, B256)> for NumHash {
    fn from(val: (u64, B256)) -> Self {
        Self { number: val.0, hash: val.1 }
    }
}

impl From<(B256, u64)> for NumHash {
    fn from(val: (B256, u64)) -> Self {
        Self { hash: val.0, number: val.1 }
    }
}

/// Either a hash _or_ a block number
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum HashOrNumber {
    /// The hash
    Hash(B256),
    /// The number
    Number(u64),
}

/// A block hash _or_ a block number
pub type BlockHashOrNumber = HashOrNumber;

// === impl HashOrNumber ===

impl HashOrNumber {
    /// Returns the block number if it is a [`HashOrNumber::Number`].
    #[inline]
    pub const fn as_number(self) -> Option<u64> {
        match self {
            Self::Hash(_) => None,
            Self::Number(num) => Some(num),
        }
    }

    /// Returns the block hash if it is a [`HashOrNumber::Hash`].
    #[inline]
    pub const fn as_hash(self) -> Option<B256> {
        match self {
            Self::Hash(hash) => Some(hash),
            Self::Number(_) => None,
        }
    }
}

impl From<B256> for HashOrNumber {
    fn from(value: B256) -> Self {
        Self::Hash(value)
    }
}

impl From<&B256> for HashOrNumber {
    fn from(value: &B256) -> Self {
        (*value).into()
    }
}

impl From<u64> for HashOrNumber {
    fn from(value: u64) -> Self {
        Self::Number(value)
    }
}

impl From<U64> for HashOrNumber {
    fn from(value: U64) -> Self {
        value.to::<u64>().into()
    }
}

impl From<RpcBlockHash> for HashOrNumber {
    fn from(value: RpcBlockHash) -> Self {
        Self::Hash(value.into())
    }
}

/// Allows for RLP encoding of either a hash or a number
impl Encodable for HashOrNumber {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        match self {
            Self::Hash(block_hash) => block_hash.encode(out),
            Self::Number(block_number) => block_number.encode(out),
        }
    }
    fn length(&self) -> usize {
        match self {
            Self::Hash(block_hash) => block_hash.length(),
            Self::Number(block_number) => block_number.length(),
        }
    }
}

/// Allows for RLP decoding of a hash or number
impl Decodable for HashOrNumber {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header: u8 = *buf.first().ok_or(RlpError::InputTooShort)?;
        // if the byte string is exactly 32 bytes, decode it into a Hash
        // 0xa0 = 0x80 (start of string) + 0x20 (32, length of string)
        if header == 0xa0 {
            // strip the first byte, parsing the rest of the string.
            // If the rest of the string fails to decode into 32 bytes, we'll bubble up the
            // decoding error.
            Ok(B256::decode(buf)?.into())
        } else {
            // a block number when encoded as bytes ranges from 0 to any number of bytes - we're
            // going to accept numbers which fit in less than 64 bits.
            // Any data larger than this which is not caught by the Hash decoding should error and
            // is considered an invalid block number.
            Ok(u64::decode(buf)?.into())
        }
    }
}

impl fmt::Display for HashOrNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(hash) => write!(f, "{hash}"),
            Self::Number(num) => write!(f, "{num}"),
        }
    }
}

/// Error thrown when parsing a [HashOrNumber] from a string.
#[derive(Debug)]
pub struct ParseBlockHashOrNumberError {
    input: alloc::string::String,
    parse_int_error: ParseIntError,
    hex_error: alloy_primitives::hex::FromHexError,
}

impl fmt::Display for ParseBlockHashOrNumberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to parse {:?} as a number: {} or hash: {}",
            self.input, self.parse_int_error, self.hex_error
        )
    }
}

impl core::error::Error for ParseBlockHashOrNumberError {}

impl FromStr for HashOrNumber {
    type Err = ParseBlockHashOrNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use alloc::string::ToString;

        match u64::from_str(s) {
            Ok(val) => Ok(val.into()),
            Err(parse_int_error) => match B256::from_str(s) {
                Ok(val) => Ok(val.into()),
                Err(hex_error) => Err(ParseBlockHashOrNumberError {
                    input: s.to_string(),
                    parse_int_error,
                    hex_error,
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::ToString, vec::Vec};
    use alloy_primitives::b256;

    const HASH: B256 = b256!("1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9");

    #[test]
    fn block_id_from_str() {
        for (s, expected) in [
            ("0x0", BlockId::number(0)),
            ("0x24A931", BlockId::number(2402609)),
            ("0x12345", BlockId::number(74565)),
            ("12345", BlockId::number(12345)),
            ("0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9", HASH.into()),
            ("latest", BlockId::latest()),
            ("finalized", BlockId::finalized()),
            ("safe", BlockId::safe()),
            ("earliest", BlockId::earliest()),
            ("pending", BlockId::pending()),
        ] {
            assert_eq!(s.parse::<BlockId>().unwrap(), expected, "{s}");
        }
        assert!("invalid_block_id".parse::<BlockId>().is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn block_number_or_tag_serialization() {
        let number = BlockNumberOrTag::Number(0);
        assert_eq!(serde_json::to_string(&number).unwrap(), "\"0x0\"");

        let number = BlockNumberOrTag::Number(16);
        assert_eq!(serde_json::to_string(&number).unwrap(), "\"0x10\"");

        let latest = BlockNumberOrTag::Latest;
        assert_eq!(serde_json::to_string(&latest).unwrap(), "\"latest\"");

        let pending = BlockNumberOrTag::Pending;
        assert_eq!(serde_json::to_string(&pending).unwrap(), "\"pending\"");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn can_parse_eip1898_block_ids() {
        let hash = b256!("d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3");
        let mut cases = vec![
            (serde_json::json!({ "blockNumber": "0x0" }), BlockId::number(0)),
            (serde_json::json!({ "blockNumber": "0xaf" }), BlockId::number(175)),
            (serde_json::json!("0x0"), BlockId::number(0)),
            (serde_json::json!({ "blockHash": hash }), BlockId::hash(hash)),
            (
                serde_json::json!({ "blockHash": hash, "requireCanonical": true }),
                BlockId::hash_canonical(hash),
            ),
        ];
        for (name, tag) in [
            ("pending", BlockNumberOrTag::Pending),
            ("latest", BlockNumberOrTag::Latest),
            ("finalized", BlockNumberOrTag::Finalized),
            ("safe", BlockNumberOrTag::Safe),
            ("earliest", BlockNumberOrTag::Earliest),
        ] {
            cases.push((serde_json::json!({ "blockNumber": name }), tag.into()));
            cases.push((serde_json::json!(name), tag.into()));
        }

        for (json, expected) in cases {
            assert_eq!(
                serde_json::from_value::<BlockId>(json.clone()).unwrap(),
                expected,
                "{json}"
            );
            assert_eq!(
                serde_json::from_str::<BlockId>(&json.to_string()).unwrap(),
                expected,
                "{json}"
            );
        }
    }

    #[test]
    fn display_rpc_block_hash() {
        let hash = RpcBlockHash::from_hash(HASH, Some(true));

        assert_eq!(
            hash.to_string(),
            "canonical hash 0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9"
        );

        let hash = RpcBlockHash::from_hash(HASH, None);

        assert_eq!(
            hash.to_string(),
            "hash 0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9"
        );
    }

    #[test]
    fn display_block_id() {
        let id = BlockId::hash(HASH);

        assert_eq!(
            id.to_string(),
            "hash 0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9"
        );

        let id = BlockId::hash_canonical(HASH);

        assert_eq!(
            id.to_string(),
            "canonical hash 0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9"
        );

        let id = BlockId::number(100000);

        assert_eq!(id.to_string(), "0x186a0");

        let id = BlockId::latest();

        assert_eq!(id.to_string(), "latest");

        let id = BlockId::safe();

        assert_eq!(id.to_string(), "safe");

        let id = BlockId::finalized();

        assert_eq!(id.to_string(), "finalized");

        let id = BlockId::earliest();

        assert_eq!(id.to_string(), "earliest");

        let id = BlockId::pending();

        assert_eq!(id.to_string(), "pending");
    }

    #[test]
    fn hash_or_number_rlp_roundtrip() {
        for hash_or_number in [HashOrNumber::Hash(B256::random()), HashOrNumber::Number(12345)] {
            let mut buf = Vec::new();
            hash_or_number.encode(&mut buf);
            assert_eq!(HashOrNumber::decode(&mut &buf[..]).unwrap(), hash_or_number);
        }
    }

    #[test]
    fn test_numhash_matches_block_or_num() {
        let number: u64 = 42;
        let hash = B256::random();

        let num_hash = NumHash::new(number, hash);

        // Test matching by hash
        let block_hash = HashOrNumber::Hash(hash);
        assert!(num_hash.matches_block_or_num(&block_hash));

        // Test matching by number
        let block_number = HashOrNumber::Number(number);
        assert!(num_hash.matches_block_or_num(&block_number));

        // Test non-matching by different hash
        let different_hash = B256::random();
        let non_matching_hash = HashOrNumber::Hash(different_hash);
        assert!(!num_hash.matches_block_or_num(&non_matching_hash));

        // Test non-matching by different number
        let different_number: u64 = 43;
        let non_matching_number = HashOrNumber::Number(different_number);
        assert!(!num_hash.matches_block_or_num(&non_matching_number));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn repeated_keys_is_err() {
        let num = serde_json::json!({"blockNumber": 1, "requireCanonical": true, "requireCanonical": false});
        assert!(serde_json::from_value::<BlockId>(num).is_err());
        let num =
            serde_json::json!({"blockNumber": 1, "requireCanonical": true, "blockNumber": 23});
        assert!(serde_json::from_value::<BlockId>(num).is_err());
    }

    /// Serde tests
    #[test]
    #[cfg(feature = "serde")]
    fn serde_blockid_tags() {
        let block_ids = [
            BlockNumberOrTag::Latest,
            BlockNumberOrTag::Finalized,
            BlockNumberOrTag::Safe,
            BlockNumberOrTag::Pending,
        ]
        .map(BlockId::from);
        for block_id in &block_ids {
            let serialized = serde_json::to_string(&block_id).unwrap();
            let deserialized: BlockId = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, *block_id)
        }
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blockid_number() {
        for (number, expected) in [(0u64, "\"0x0\""), (100, "\"0x64\"")] {
            let block_id = BlockId::from(number);
            let serialized = serde_json::to_string(&block_id).unwrap();
            assert_eq!(serialized, expected);
            let deserialized: BlockId = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, block_id)
        }
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blockid_hash() {
        let block_id = BlockId::from(B256::default());
        let serialized = serde_json::to_string(&block_id).unwrap();
        assert_eq!(
            serialized,
            "\"0x0000000000000000000000000000000000000000000000000000000000000000\""
        );
        let deserialized: BlockId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, block_id)
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blockid_hash_param() {
        let params = (BlockId::hash(HASH),);
        let serialized = serde_json::to_string(&params).unwrap();
        assert_eq!(
            serialized,
            "[\"0x1a15e3c30cf094a99826869517b16d185d45831d3a494f01030b0001a9d3ebb9\"]"
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blockid_canonical_hash() {
        for require_canonical in [true, false] {
            let block_id = BlockId::from((HASH, Some(require_canonical)));
            let serialized = serde_json::to_value(block_id).unwrap();
            assert_eq!(
                serialized,
                serde_json::json!({
                    "blockHash": HASH,
                    "requireCanonical": require_canonical,
                })
            );
            let deserialized: BlockId = serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized, block_id);
        }
    }

    #[test]
    #[should_panic]
    #[cfg(feature = "serde")]
    fn serde_rpc_payload_block_number_duplicate_key() {
        let payload = r#"{"blockNumber": "0x132", "blockNumber": "0x133"}"#;
        let parsed_block_id = serde_json::from_str::<BlockId>(payload);
        parsed_block_id.unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_blocknumber_non_0xprefix() {
        let s = "\"2\"";
        let err = serde_json::from_str::<BlockNumberOrTag>(s).unwrap_err();
        assert_eq!(err.to_string(), HexStringMissingPrefixError::default().to_string());
    }

    #[cfg(feature = "serde")]
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct TestLenientStruct {
        #[serde(deserialize_with = "super::lenient_block_number_or_tag::deserialize")]
        block: BlockNumberOrTag,
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_lenient_block_number_or_tag() {
        // Test parsing numeric strings
        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "0x1"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Number(1));

        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "123"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Number(123));

        // Test parsing tags
        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "latest"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Latest);

        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "finalized"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Finalized);

        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "safe"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Safe);

        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "earliest"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Earliest);

        let lenient_struct: TestLenientStruct =
            serde_json::from_str(r#"{"block": "pending"}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Pending);

        // Test parsing raw numbers (not strings)
        let lenient_struct: TestLenientStruct = serde_json::from_str(r#"{"block": 123}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Number(123));

        let lenient_struct: TestLenientStruct = serde_json::from_str(r#"{"block": 0}"#).unwrap();
        assert_eq!(lenient_struct.block, BlockNumberOrTag::Number(0));

        // Test invalid inputs
        assert!(serde_json::from_str::<TestLenientStruct>(r#"{"block": "invalid"}"#).is_err());
        assert!(serde_json::from_str::<TestLenientStruct>(r#"{"block": null}"#).is_err());
        assert!(serde_json::from_str::<TestLenientStruct>(r#"{"block": {}}"#).is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_lenient_block_number_or_tag_opt() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct TestLenientOptStruct {
            #[serde(
                default,
                deserialize_with = "super::lenient_block_number_or_tag::opt::deserialize"
            )]
            block: Option<BlockNumberOrTag>,
        }

        for (raw, expected) in [
            (r#"{"block": 123}"#, Some(BlockNumberOrTag::Number(123))),
            (r#"{"block": "123"}"#, Some(BlockNumberOrTag::Number(123))),
            (r#"{"block": "latest"}"#, Some(BlockNumberOrTag::Latest)),
            (r#"{"block": null}"#, None),
            ("{}", None),
        ] {
            let lenient_struct: TestLenientOptStruct = serde_json::from_str(raw).unwrap();
            assert_eq!(lenient_struct.block, expected);
        }
        assert!(serde_json::from_str::<TestLenientOptStruct>(r#"{"block": "invalid"}"#).is_err());
    }
}
