//! Block RPC types.

#![allow(unknown_lints, non_local_definitions)]

use crate::{other::OtherFields, Transaction, Withdrawal};
pub use alloy_eips::{
    calc_blob_gasprice, calc_excess_blob_gas, BlockId, BlockNumberOrTag, RpcBlockHash,
};
use alloy_primitives::{Address, BlockHash, BlockNumber, Bloom, Bytes, B256, B64, U256, U64};
use alloy_rlp::{bytes, Decodable, Encodable, Error as RlpError};
use serde::{ser::Error, Deserialize, Serialize, Serializer};
use std::{collections::BTreeMap, fmt, num::ParseIntError, ops::Deref, str::FromStr};

/// Block representation
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Header of the block.
    #[serde(flatten)]
    pub header: Header,
    /// Uncles' hashes.
    #[serde(default)]
    pub uncles: Vec<B256>,
    /// Block Transactions. In the case of an uncle block, this field is not included in RPC
    /// responses, and when deserialized, it will be set to [BlockTransactions::Uncle].
    #[serde(
        default = "BlockTransactions::uncle",
        skip_serializing_if = "BlockTransactions::is_uncle"
    )]
    pub transactions: BlockTransactions,
    /// Integer the size of this block in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<U256>,
    /// Withdrawals in the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Support for arbitrary additional fields.
    #[serde(flatten)]
    pub other: OtherFields,
}

impl Block {
    /// Converts a block with Tx hashes into a full block.
    pub fn into_full_block(self, txs: Vec<Transaction>) -> Self {
        Self { transactions: BlockTransactions::Full(txs), ..self }
    }
}

/// Block header representation.
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Hash of the block
    pub hash: Option<B256>,
    /// Hash of the parent
    pub parent_hash: B256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: B256,
    /// Alias of `author`
    pub miner: Address,
    /// State root hash
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
    #[serde(default, with = "alloy_serde::num::u64_opt_via_ruint")]
    pub number: Option<u64>,
    /// Gas Limit
    #[serde(default, with = "alloy_serde::num::u128_via_ruint")]
    pub gas_limit: u128,
    /// Gas Used
    #[serde(default, with = "alloy_serde::num::u128_via_ruint")]
    pub gas_used: u128,
    /// Timestamp
    #[serde(default, with = "alloy_serde::num::u64_via_ruint")]
    pub timestamp: u64,
    /// Total difficulty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_difficulty: Option<U256>,
    /// Extra data
    pub extra_data: Bytes,
    /// Mix Hash
    ///
    /// Before the merge this proves, combined with the nonce, that a sufficient amount of
    /// computation has been carried out on this block: the Proof-of-Work (PoF).
    ///
    /// After the merge this is `prevRandao`: Randomness value for the generated payload.
    ///
    /// This is an Option because it is not always set by non-ethereum networks.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-4399>
    /// And <https://github.com/ethereum/execution-apis/issues/328>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mix_hash: Option<B256>,
    /// Nonce
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<B64>,
    /// Base fee per unit of gas (if past London)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub base_fee_per_gas: Option<u128>,
    /// Withdrawals root hash added by EIP-4895 and is ignored in legacy headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawals_root: Option<B256>,
    /// Blob gas used
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub blob_gas_used: Option<u128>,
    /// Excess blob gas
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub excess_blob_gas: Option<u128>,
    /// Parent beacon block root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_beacon_block_root: Option<B256>,
}

impl Header {
    /// Returns the blob fee for _this_ block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None
    pub fn blob_fee(&self) -> Option<u128> {
        self.excess_blob_gas.map(calc_blob_gasprice)
    }

    /// Returns the blob fee for the next block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None.
    ///
    /// See also [Self::next_block_excess_blob_gas]
    pub fn next_block_blob_fee(&self) -> Option<u128> {
        self.next_block_excess_blob_gas().map(calc_blob_gasprice)
    }

    /// Calculate excess blob gas for the next block according to the EIP-4844
    /// spec.
    ///
    /// Returns a `None` if no excess blob gas is set, no EIP-4844 support
    pub fn next_block_excess_blob_gas(&self) -> Option<u128> {
        Some(calc_excess_blob_gas(self.excess_blob_gas?, self.blob_gas_used?))
    }
}

/// Block Transactions depending on the boolean attribute of `eth_getBlockBy*`,
/// or if used by `eth_getUncle*`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockTransactions<T = Transaction> {
    /// Only hashes
    Hashes(Vec<B256>),
    /// Full transactions
    Full(Vec<T>),
    /// Special case for uncle response.
    Uncle,
}

impl Default for BlockTransactions {
    fn default() -> Self {
        BlockTransactions::Hashes(Vec::default())
    }
}

impl BlockTransactions {
    /// Converts `self` into `Hashes`.
    #[inline]
    pub fn convert_to_hashes(&mut self) {
        if !self.is_hashes() {
            *self = Self::Hashes(self.hashes().copied().collect());
        }
    }

    /// Converts `self` into `Hashes`.
    #[inline]
    pub fn into_hashes(mut self) -> Self {
        self.convert_to_hashes();
        self
    }

    /// Check if the enum variant is used for hashes.
    #[inline]
    pub const fn is_hashes(&self) -> bool {
        matches!(self, Self::Hashes(_))
    }

    /// Returns true if the enum variant is used for full transactions.
    #[inline]
    pub const fn is_full(&self) -> bool {
        matches!(self, Self::Full(_))
    }

    /// Returns true if the enum variant is used for an uncle response.
    #[inline]
    pub const fn is_uncle(&self) -> bool {
        matches!(self, Self::Uncle)
    }

    /// Returns an iterator over the transaction hashes.
    #[deprecated = "use `hashes` instead"]
    #[inline]
    pub fn iter(&self) -> BlockTransactionHashes<'_> {
        self.hashes()
    }

    /// Returns an iterator over references to the transaction hashes.
    #[inline]
    pub fn hashes(&self) -> BlockTransactionHashes<'_> {
        BlockTransactionHashes::new(self)
    }

    /// Returns an iterator over mutable references to the transaction hashes.
    #[inline]
    pub fn hashes_mut(&mut self) -> BlockTransactionHashesMut<'_> {
        BlockTransactionHashesMut::new(self)
    }

    /// Returns an instance of BlockTransactions with the Uncle special case.
    #[inline]
    pub const fn uncle() -> Self {
        Self::Uncle
    }

    /// Returns the number of transactions.
    #[inline]
    pub fn len(&self) -> usize {
        self.hashes().len()
    }

    /// Whether the block has no transactions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An iterator over the transaction hashes of a block.
///
/// See [`BlockTransactions::hashes`].
#[derive(Clone, Debug)]
pub struct BlockTransactionHashes<'a>(BlockTransactionHashesInner<'a>);

#[derive(Clone, Debug)]
enum BlockTransactionHashesInner<'a> {
    Hashes(std::slice::Iter<'a, B256>),
    Full(std::slice::Iter<'a, Transaction>),
    Uncle,
}

impl<'a> BlockTransactionHashes<'a> {
    #[inline]
    fn new(txs: &'a BlockTransactions) -> Self {
        Self(match txs {
            BlockTransactions::Hashes(txs) => BlockTransactionHashesInner::Hashes(txs.iter()),
            BlockTransactions::Full(txs) => BlockTransactionHashesInner::Full(txs.iter()),
            BlockTransactions::Uncle => BlockTransactionHashesInner::Uncle,
        })
    }
}

impl<'a> Iterator for BlockTransactionHashes<'a> {
    type Item = &'a B256;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            BlockTransactionHashesInner::Full(txs) => txs.next().map(|tx| &tx.hash),
            BlockTransactionHashesInner::Hashes(txs) => txs.next(),
            BlockTransactionHashesInner::Uncle => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            BlockTransactionHashesInner::Full(txs) => txs.size_hint(),
            BlockTransactionHashesInner::Hashes(txs) => txs.size_hint(),
            BlockTransactionHashesInner::Uncle => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for BlockTransactionHashes<'_> {
    #[inline]
    fn len(&self) -> usize {
        match &self.0 {
            BlockTransactionHashesInner::Full(txs) => txs.len(),
            BlockTransactionHashesInner::Hashes(txs) => txs.len(),
            BlockTransactionHashesInner::Uncle => 0,
        }
    }
}

impl DoubleEndedIterator for BlockTransactionHashes<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            BlockTransactionHashesInner::Full(txs) => txs.next_back().map(|tx| &tx.hash),
            BlockTransactionHashesInner::Hashes(txs) => txs.next_back(),
            BlockTransactionHashesInner::Uncle => None,
        }
    }
}

impl<'a> std::iter::FusedIterator for BlockTransactionHashes<'a> {}

/// An Iterator over the transaction hashes of a block.
///
/// See [`BlockTransactions::hashes_mut`].
#[derive(Debug)]
pub struct BlockTransactionHashesMut<'a>(BlockTransactionHashesInnerMut<'a>);

#[derive(Debug)]
enum BlockTransactionHashesInnerMut<'a> {
    Hashes(std::slice::IterMut<'a, B256>),
    Full(std::slice::IterMut<'a, Transaction>),
    Uncle,
}

impl<'a> BlockTransactionHashesMut<'a> {
    #[inline]
    fn new(txs: &'a mut BlockTransactions) -> Self {
        Self(match txs {
            BlockTransactions::Hashes(txs) => {
                BlockTransactionHashesInnerMut::Hashes(txs.iter_mut())
            }
            BlockTransactions::Full(txs) => BlockTransactionHashesInnerMut::Full(txs.iter_mut()),
            BlockTransactions::Uncle => BlockTransactionHashesInnerMut::Uncle,
        })
    }
}

impl<'a> Iterator for BlockTransactionHashesMut<'a> {
    type Item = &'a mut B256;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            BlockTransactionHashesInnerMut::Full(txs) => txs.next().map(|tx| &mut tx.hash),
            BlockTransactionHashesInnerMut::Hashes(txs) => txs.next(),
            BlockTransactionHashesInnerMut::Uncle => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            BlockTransactionHashesInnerMut::Full(txs) => txs.size_hint(),
            BlockTransactionHashesInnerMut::Hashes(txs) => txs.size_hint(),
            BlockTransactionHashesInnerMut::Uncle => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for BlockTransactionHashesMut<'_> {
    #[inline]
    fn len(&self) -> usize {
        match &self.0 {
            BlockTransactionHashesInnerMut::Full(txs) => txs.len(),
            BlockTransactionHashesInnerMut::Hashes(txs) => txs.len(),
            BlockTransactionHashesInnerMut::Uncle => 0,
        }
    }
}

impl DoubleEndedIterator for BlockTransactionHashesMut<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            BlockTransactionHashesInnerMut::Full(txs) => txs.next_back().map(|tx| &mut tx.hash),
            BlockTransactionHashesInnerMut::Hashes(txs) => txs.next_back(),
            BlockTransactionHashesInnerMut::Uncle => None,
        }
    }
}

impl<'a> std::iter::FusedIterator for BlockTransactionHashesMut<'a> {}

/// Determines how the `transactions` field of [Block] should be filled.
///
/// This essentially represents the `full:bool` argument in RPC calls that determine whether the
/// response should include full transaction objects or just the hashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockTransactionsKind {
    /// Only include hashes: [BlockTransactions::Hashes]
    Hashes,
    /// Include full transaction objects: [BlockTransactions::Full]
    Full,
}

impl From<bool> for BlockTransactionsKind {
    fn from(is_full: bool) -> Self {
        if is_full {
            BlockTransactionsKind::Full
        } else {
            BlockTransactionsKind::Hashes
        }
    }
}

/// Error that can occur when converting other types to blocks
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum BlockError {
    /// A transaction failed sender recovery
    #[error("transaction failed sender recovery")]
    InvalidSignature,
    /// A raw block failed to decode
    #[error("failed to decode raw block {0}")]
    RlpDecodeRawBlock(alloy_rlp::Error),
}

/// Block number and hash.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BlockNumHash {
    /// Block number
    pub number: BlockNumber,
    /// Block hash
    pub hash: BlockHash,
}

/// Block number and hash of the forked block.
pub type ForkBlock = BlockNumHash;

impl fmt::Debug for BlockNumHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").field(&self.number).field(&self.hash).finish()
    }
}

impl BlockNumHash {
    /// Creates a new `BlockNumHash` from a block number and hash.
    pub const fn new(number: BlockNumber, hash: BlockHash) -> Self {
        Self { number, hash }
    }

    /// Consumes `Self` and returns [`BlockNumber`], [`BlockHash`]
    pub const fn into_components(self) -> (BlockNumber, BlockHash) {
        (self.number, self.hash)
    }

    /// Returns whether or not the block matches the given [BlockHashOrNumber].
    pub fn matches_block_or_num(&self, block: &BlockHashOrNumber) -> bool {
        match block {
            BlockHashOrNumber::Hash(hash) => self.hash == *hash,
            BlockHashOrNumber::Number(number) => self.number == *number,
        }
    }
}

impl From<(BlockNumber, BlockHash)> for BlockNumHash {
    fn from(val: (BlockNumber, BlockHash)) -> Self {
        BlockNumHash { number: val.0, hash: val.1 }
    }
}

impl From<(BlockHash, BlockNumber)> for BlockNumHash {
    fn from(val: (BlockHash, BlockNumber)) -> Self {
        BlockNumHash { hash: val.0, number: val.1 }
    }
}

/// Either a block hash _or_ a block number
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
pub enum BlockHashOrNumber {
    /// A block hash
    Hash(B256),
    /// A block number
    Number(u64),
}

// === impl BlockHashOrNumber ===

impl BlockHashOrNumber {
    /// Returns the block number if it is a [`BlockHashOrNumber::Number`].
    #[inline]
    pub const fn as_number(self) -> Option<u64> {
        match self {
            BlockHashOrNumber::Hash(_) => None,
            BlockHashOrNumber::Number(num) => Some(num),
        }
    }
}

impl From<B256> for BlockHashOrNumber {
    fn from(value: B256) -> Self {
        BlockHashOrNumber::Hash(value)
    }
}

impl From<u64> for BlockHashOrNumber {
    fn from(value: u64) -> Self {
        BlockHashOrNumber::Number(value)
    }
}

impl From<U64> for BlockHashOrNumber {
    fn from(value: U64) -> Self {
        value.to::<u64>().into()
    }
}

/// Allows for RLP encoding of either a block hash or block number
impl Encodable for BlockHashOrNumber {
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

/// Allows for RLP decoding of a block hash or block number
impl Decodable for BlockHashOrNumber {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header: u8 = *buf.first().ok_or(RlpError::InputTooShort)?;
        // if the byte string is exactly 32 bytes, decode it into a Hash
        // 0xa0 = 0x80 (start of string) + 0x20 (32, length of string)
        if header == 0xa0 {
            // strip the first byte, parsing the rest of the string.
            // If the rest of the string fails to decode into 32 bytes, we'll bubble up the
            // decoding error.
            let hash = B256::decode(buf)?;
            Ok(Self::Hash(hash))
        } else {
            // a block number when encoded as bytes ranges from 0 to any number of bytes - we're
            // going to accept numbers which fit in less than 64 bytes.
            // Any data larger than this which is not caught by the Hash decoding should error and
            // is considered an invalid block number.
            Ok(Self::Number(u64::decode(buf)?))
        }
    }
}

impl fmt::Display for BlockHashOrNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hash(hash) => write!(f, "{}", hash),
            Self::Number(num) => write!(f, "{}", num),
        }
    }
}

/// Error thrown when parsing a [BlockHashOrNumber] from a string.
#[derive(Debug, thiserror::Error)]
#[error("failed to parse {input:?} as a number: {parse_int_error} or hash: {hex_error}")]
pub struct ParseBlockHashOrNumberError {
    input: String,
    parse_int_error: ParseIntError,
    hex_error: alloy_primitives::hex::FromHexError,
}

impl FromStr for BlockHashOrNumber {
    type Err = ParseBlockHashOrNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match u64::from_str(s) {
            Ok(val) => Ok(val.into()),
            Err(pares_int_error) => match B256::from_str(s) {
                Ok(val) => Ok(val.into()),
                Err(hex_error) => Err(ParseBlockHashOrNumberError {
                    input: s.to_string(),
                    parse_int_error: pares_int_error,
                    hex_error,
                }),
            },
        }
    }
}

/// A Block representation that allows to include additional fields
pub type RichBlock = Rich<Block>;

impl From<Block> for RichBlock {
    fn from(block: Block) -> Self {
        Rich { inner: block, extra_info: Default::default() }
    }
}

/// Header representation with additional info.
pub type RichHeader = Rich<Header>;

impl From<Header> for RichHeader {
    fn from(header: Header) -> Self {
        Rich { inner: header, extra_info: Default::default() }
    }
}

/// Value representation with additional info
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Rich<T> {
    /// Standard value.
    #[serde(flatten)]
    pub inner: T,
    /// Additional fields that should be serialized into the `Block` object
    #[serde(flatten)]
    pub extra_info: BTreeMap<String, serde_json::Value>,
}

impl<T> Deref for Rich<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Serialize> Serialize for Rich<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.extra_info.is_empty() {
            return self.inner.serialize(serializer);
        }

        let inner = serde_json::to_value(&self.inner);
        let extras = serde_json::to_value(&self.extra_info);

        if let (Ok(serde_json::Value::Object(mut value)), Ok(serde_json::Value::Object(extras))) =
            (inner, extras)
        {
            value.extend(extras);
            value.serialize(serializer)
        } else {
            Err(S::Error::custom("Unserializable structures: expected objects"))
        }
    }
}

/// BlockOverrides is a set of header fields to override.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockOverrides {
    /// Overrides the block number.
    ///
    /// For `eth_callMany` this will be the block number of the first simulated block. Each
    /// following block increments its block number by 1
    // Note: geth uses `number`, erigon uses `blockNumber`
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "blockNumber")]
    pub number: Option<U256>,
    /// Overrides the difficulty of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<U256>,
    /// Overrides the timestamp of the block.
    // Note: geth uses `time`, erigon uses `timestamp`
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "timestamp")]
    pub time: Option<U64>,
    /// Overrides the gas limit of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<U64>,
    /// Overrides the coinbase address of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coinbase: Option<Address>,
    /// Overrides the prevrandao of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<B256>,
    /// Overrides the basefee of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_fee: Option<U256>,
    /// A dictionary that maps blockNumber to a user-defined hash. It could be queried from the
    /// solidity opcode BLOCKHASH.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BTreeMap<u64, B256>>,
}

#[cfg(test)]
mod tests {
    use arbitrary::Arbitrary;
    use rand::Rng;

    use super::*;

    #[test]
    fn arbitrary_header() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let _: Header = Header::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn test_full_conversion() {
        let full = true;
        assert_eq!(BlockTransactionsKind::Full, full.into());

        let full = false;
        assert_eq!(BlockTransactionsKind::Hashes, full.into());
    }

    #[test]
    #[cfg(feature = "jsonrpsee-types")]
    fn serde_json_header() {
        use jsonrpsee_types::SubscriptionResponse;
        let resp = r#"{"jsonrpc":"2.0","method":"eth_subscribe","params":{"subscription":"0x7eef37ff35d471f8825b1c8f67a5d3c0","result":{"hash":"0x7a7ada12e140961a32395059597764416499f4178daf1917193fad7bd2cc6386","parentHash":"0xdedbd831f496e705e7f2ec3c8dcb79051040a360bf1455dbd7eb8ea6ad03b751","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","number":"0x8","gasUsed":"0x0","gasLimit":"0x1c9c380","extraData":"0x","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","timestamp":"0x642aa48f","difficulty":"0x0","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000"}}}"#;
        let _header: SubscriptionResponse<'_, Header> = serde_json::from_str(resp).unwrap();

        let resp = r#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0x1a14b6bdcf4542fabf71c4abee244e47","result":{"author":"0x000000568b9b5a365eaa767d42e74ed88915c204","difficulty":"0x1","extraData":"0x4e65746865726d696e6420312e392e32322d302d6463373666616366612d32308639ad8ff3d850a261f3b26bc2a55e0f3a718de0dd040a19a4ce37e7b473f2d7481448a1e1fd8fb69260825377c0478393e6055f471a5cf839467ce919a6ad2700","gasLimit":"0x7a1200","gasUsed":"0x0","hash":"0xa4856602944fdfd18c528ef93cc52a681b38d766a7e39c27a47488c8461adcb0","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","miner":"0x0000000000000000000000000000000000000000","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","number":"0x434822","parentHash":"0x1a9bdc31fc785f8a95efeeb7ae58f40f6366b8e805f47447a52335c95f4ceb49","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","size":"0x261","stateRoot":"0xf38c4bf2958e541ec6df148e54ce073dc6b610f8613147ede568cb7b5c2d81ee","totalDifficulty":"0x633ebd","timestamp":"0x604726b0","transactions":[],"transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","uncles":[]}}}"#;
        let _header: SubscriptionResponse<'_, Header> = serde_json::from_str(resp).unwrap();
    }

    #[test]
    fn serde_block() {
        let block = Block {
            header: Header {
                hash: Some(B256::with_last_byte(1)),
                parent_hash: B256::with_last_byte(2),
                uncles_hash: B256::with_last_byte(3),
                miner: Address::with_last_byte(4),
                state_root: B256::with_last_byte(5),
                transactions_root: B256::with_last_byte(6),
                receipts_root: B256::with_last_byte(7),
                withdrawals_root: Some(B256::with_last_byte(8)),
                number: Some(9),
                gas_used: 10,
                gas_limit: 11,
                extra_data: Bytes::from(vec![1, 2, 3]),
                logs_bloom: Bloom::default(),
                timestamp: 12,
                difficulty: U256::from(13),
                total_difficulty: Some(U256::from(100000)),
                mix_hash: Some(B256::with_last_byte(14)),
                nonce: Some(B64::with_last_byte(15)),
                base_fee_per_gas: Some(20),
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
            },
            uncles: vec![B256::with_last_byte(17)],
            transactions: BlockTransactions::Hashes(vec![B256::with_last_byte(18)]),
            size: Some(U256::from(19)),
            withdrawals: Some(vec![]),
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","parentHash":"0x0000000000000000000000000000000000000000000000000000000000000002","sha3Uncles":"0x0000000000000000000000000000000000000000000000000000000000000003","miner":"0x0000000000000000000000000000000000000004","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000005","transactionsRoot":"0x0000000000000000000000000000000000000000000000000000000000000006","receiptsRoot":"0x0000000000000000000000000000000000000000000000000000000000000007","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0xd","number":"0x9","gasLimit":"0xb","gasUsed":"0xa","timestamp":"0xc","totalDifficulty":"0x186a0","extraData":"0x010203","mixHash":"0x000000000000000000000000000000000000000000000000000000000000000e","nonce":"0x000000000000000f","baseFeePerGas":"0x14","withdrawalsRoot":"0x0000000000000000000000000000000000000000000000000000000000000008","uncles":["0x0000000000000000000000000000000000000000000000000000000000000011"],"transactions":["0x0000000000000000000000000000000000000000000000000000000000000012"],"size":"0x13","withdrawals":[]}"#
        );
        let deserialized: Block = serde_json::from_str(&serialized).unwrap();
        assert_eq!(block, deserialized);
    }

    #[test]
    fn serde_uncle_block() {
        let block = Block {
            header: Header {
                hash: Some(B256::with_last_byte(1)),
                parent_hash: B256::with_last_byte(2),
                uncles_hash: B256::with_last_byte(3),
                miner: Address::with_last_byte(4),
                state_root: B256::with_last_byte(5),
                transactions_root: B256::with_last_byte(6),
                receipts_root: B256::with_last_byte(7),
                withdrawals_root: Some(B256::with_last_byte(8)),
                number: Some(9),
                gas_used: 10,
                gas_limit: 11,
                extra_data: Bytes::from(vec![1, 2, 3]),
                logs_bloom: Bloom::default(),
                timestamp: 12,
                difficulty: U256::from(13),
                total_difficulty: Some(U256::from(100000)),
                mix_hash: Some(B256::with_last_byte(14)),
                nonce: Some(B64::with_last_byte(15)),
                base_fee_per_gas: Some(20),
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
            },
            uncles: vec![],
            transactions: BlockTransactions::Uncle,
            size: Some(U256::from(19)),
            withdrawals: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","parentHash":"0x0000000000000000000000000000000000000000000000000000000000000002","sha3Uncles":"0x0000000000000000000000000000000000000000000000000000000000000003","miner":"0x0000000000000000000000000000000000000004","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000005","transactionsRoot":"0x0000000000000000000000000000000000000000000000000000000000000006","receiptsRoot":"0x0000000000000000000000000000000000000000000000000000000000000007","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0xd","number":"0x9","gasLimit":"0xb","gasUsed":"0xa","timestamp":"0xc","totalDifficulty":"0x186a0","extraData":"0x010203","mixHash":"0x000000000000000000000000000000000000000000000000000000000000000e","nonce":"0x000000000000000f","baseFeePerGas":"0x14","withdrawalsRoot":"0x0000000000000000000000000000000000000000000000000000000000000008","uncles":[],"size":"0x13"}"#
        );
        let deserialized: Block = serde_json::from_str(&serialized).unwrap();
        assert_eq!(block, deserialized);
    }

    #[test]
    fn serde_block_with_withdrawals_set_as_none() {
        let block = Block {
            header: Header {
                hash: Some(B256::with_last_byte(1)),
                parent_hash: B256::with_last_byte(2),
                uncles_hash: B256::with_last_byte(3),
                miner: Address::with_last_byte(4),
                state_root: B256::with_last_byte(5),
                transactions_root: B256::with_last_byte(6),
                receipts_root: B256::with_last_byte(7),
                withdrawals_root: None,
                number: Some(9),
                gas_used: 10,
                gas_limit: 11,
                extra_data: Bytes::from(vec![1, 2, 3]),
                logs_bloom: Bloom::default(),
                timestamp: 12,
                difficulty: U256::from(13),
                total_difficulty: Some(U256::from(100000)),
                mix_hash: Some(B256::with_last_byte(14)),
                nonce: Some(B64::with_last_byte(15)),
                base_fee_per_gas: Some(20),
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
            },
            uncles: vec![B256::with_last_byte(17)],
            transactions: BlockTransactions::Hashes(vec![B256::with_last_byte(18)]),
            size: Some(U256::from(19)),
            withdrawals: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","parentHash":"0x0000000000000000000000000000000000000000000000000000000000000002","sha3Uncles":"0x0000000000000000000000000000000000000000000000000000000000000003","miner":"0x0000000000000000000000000000000000000004","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000005","transactionsRoot":"0x0000000000000000000000000000000000000000000000000000000000000006","receiptsRoot":"0x0000000000000000000000000000000000000000000000000000000000000007","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0xd","number":"0x9","gasLimit":"0xb","gasUsed":"0xa","timestamp":"0xc","totalDifficulty":"0x186a0","extraData":"0x010203","mixHash":"0x000000000000000000000000000000000000000000000000000000000000000e","nonce":"0x000000000000000f","baseFeePerGas":"0x14","uncles":["0x0000000000000000000000000000000000000000000000000000000000000011"],"transactions":["0x0000000000000000000000000000000000000000000000000000000000000012"],"size":"0x13"}"#
        );
        let deserialized: Block = serde_json::from_str(&serialized).unwrap();
        assert_eq!(block, deserialized);
    }

    #[test]
    fn block_overrides() {
        let s = r#"{"blockNumber": "0xe39dd0"}"#;
        let _overrides = serde_json::from_str::<BlockOverrides>(s).unwrap();
    }

    #[test]
    fn serde_rich_block() {
        let s = r#"{
    "hash": "0xb25d0e54ca0104e3ebfb5a1dcdf9528140854d609886a300946fd6750dcb19f4",
    "parentHash": "0x9400ec9ef59689c157ac89eeed906f15ddd768f94e1575e0e27d37c241439a5d",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "miner": "0x829bd824b016326a401d083b33d092293333a830",
    "stateRoot": "0x546e330050c66d02923e7f1f3e925efaf64e4384eeecf2288f40088714a77a84",
    "transactionsRoot": "0xd5eb3ad6d7c7a4798cc5fb14a6820073f44a941107c5d79dac60bd16325631fe",
    "receiptsRoot": "0xb21c41cbb3439c5af25304e1405524c885e733b16203221900cb7f4b387b62f0",
    "logsBloom": "0x1f304e641097eafae088627298685d20202004a4a59e4d8900914724e2402b028c9d596660581f361240816e82d00fa14250c9ca89840887a381efa600288283d170010ab0b2a0694c81842c2482457e0eb77c2c02554614007f42aaf3b4dc15d006a83522c86a240c06d241013258d90540c3008888d576a02c10120808520a2221110f4805200302624d22092b2c0e94e849b1e1aa80bc4cc3206f00b249d0a603ee4310216850e47c8997a20aa81fe95040a49ca5a420464600e008351d161dc00d620970b6a801535c218d0b4116099292000c08001943a225d6485528828110645b8244625a182c1a88a41087e6d039b000a180d04300d0680700a15794",
    "difficulty": "0xc40faff9c737d",
    "number": "0xa9a230",
    "gasLimit": "0xbe5a66",
    "gasUsed": "0xbe0fcc",
    "timestamp": "0x5f93b749",
    "totalDifficulty": "0x3dc957fd8167fb2684a",
    "extraData": "0x7070796520e4b883e5bda9e7a59ee4bb99e9b1bc0103",
    "mixHash": "0xd5e2b7b71fbe4ddfe552fb2377bf7cddb16bbb7e185806036cee86994c6e97fc",
    "nonce": "0x4722f2acd35abe0f",
    "uncles": [],
    "transactions": [
        "0xf435a26acc2a9ef73ac0b73632e32e29bd0e28d5c4f46a7e18ed545c93315916"
    ],
    "size": "0xaeb6"
}"#;

        let block = serde_json::from_str::<RichBlock>(s).unwrap();
        let serialized = serde_json::to_string(&block).unwrap();
        let block2 = serde_json::from_str::<RichBlock>(&serialized).unwrap();
        assert_eq!(block, block2);
    }

    #[test]
    fn serde_missing_uncles_block() {
        let s = r#"{
            "baseFeePerGas":"0x886b221ad",
            "blobGasUsed":"0x0",
            "difficulty":"0x0",
            "excessBlobGas":"0x0",
            "extraData":"0x6265617665726275696c642e6f7267",
            "gasLimit":"0x1c9c380",
            "gasUsed":"0xb0033c",
            "hash":"0x85cdcbe36217fd57bf2c33731d8460657a7ce512401f49c9f6392c82a7ccf7ac",
            "logsBloom":"0xc36919406572730518285284f2293101104140c0d42c4a786c892467868a8806f40159d29988002870403902413a1d04321320308da2e845438429e0012a00b419d8ccc8584a1c28f82a415d04eab8a5ae75c00d07761acf233414c08b6d9b571c06156086c70ea5186e9b989b0c2d55c0213c936805cd2ab331589c90194d070c00867549b1e1be14cb24500b0386cd901197c1ef5a00da453234fa48f3003dcaa894e3111c22b80e17f7d4388385a10720cda1140c0400f9e084ca34fc4870fb16b472340a2a6a63115a82522f506c06c2675080508834828c63defd06bc2331b4aa708906a06a560457b114248041e40179ebc05c6846c1e922125982f427",
            "miner":"0x95222290dd7278aa3ddd389cc1e1d165cc4bafe5",
            "mixHash":"0x4c068e902990f21f92a2456fc75c59bec8be03b7f13682b6ebd27da56269beb5",
            "nonce":"0x0000000000000000",
            "number":"0x128c6df",
            "parentBeaconBlockRoot":"0x2843cb9f7d001bd58816a915e685ed96a555c9aeec1217736bd83a96ebd409cc",
            "parentHash":"0x90926e0298d418181bd20c23b332451e35fd7d696b5dcdc5a3a0a6b715f4c717",
            "receiptsRoot":"0xd43aa19ecb03571d1b86d89d9bb980139d32f2f2ba59646cd5c1de9e80c68c90",
            "sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            "size":"0xdcc3",
            "stateRoot":"0x707875120a7103621fb4131df59904cda39de948dfda9084a1e3da44594d5404",
            "timestamp":"0x65f5f4c3",
            "transactionsRoot":"0x889a1c26dc42ba829dab552b779620feac231cde8a6c79af022bdc605c23a780",
            "withdrawals":[
               {
                  "index":"0x24d80e6",
                  "validatorIndex":"0x8b2b6",
                  "address":"0x7cd1122e8e118b12ece8d25480dfeef230da17ff",
                  "amount":"0x1161f10"
               }
            ],
            "withdrawalsRoot":"0x360c33f20eeed5efbc7d08be46e58f8440af5db503e40908ef3d1eb314856ef7"
         }"#;

        let block = serde_json::from_str::<Block>(s).unwrap();
        let serialized = serde_json::to_string(&block).unwrap();
        let block2 = serde_json::from_str::<Block>(&serialized).unwrap();
        assert_eq!(block, block2);
    }

    #[test]
    fn serde_block_containing_uncles() {
        let s = r#"{
            "baseFeePerGas":"0x886b221ad",
            "blobGasUsed":"0x0",
            "difficulty":"0x0",
            "excessBlobGas":"0x0",
            "extraData":"0x6265617665726275696c642e6f7267",
            "gasLimit":"0x1c9c380",
            "gasUsed":"0xb0033c",
            "hash":"0x85cdcbe36217fd57bf2c33731d8460657a7ce512401f49c9f6392c82a7ccf7ac",
            "logsBloom":"0xc36919406572730518285284f2293101104140c0d42c4a786c892467868a8806f40159d29988002870403902413a1d04321320308da2e845438429e0012a00b419d8ccc8584a1c28f82a415d04eab8a5ae75c00d07761acf233414c08b6d9b571c06156086c70ea5186e9b989b0c2d55c0213c936805cd2ab331589c90194d070c00867549b1e1be14cb24500b0386cd901197c1ef5a00da453234fa48f3003dcaa894e3111c22b80e17f7d4388385a10720cda1140c0400f9e084ca34fc4870fb16b472340a2a6a63115a82522f506c06c2675080508834828c63defd06bc2331b4aa708906a06a560457b114248041e40179ebc05c6846c1e922125982f427",
            "miner":"0x95222290dd7278aa3ddd389cc1e1d165cc4bafe5",
            "mixHash":"0x4c068e902990f21f92a2456fc75c59bec8be03b7f13682b6ebd27da56269beb5",
            "nonce":"0x0000000000000000",
            "number":"0x128c6df",
            "parentBeaconBlockRoot":"0x2843cb9f7d001bd58816a915e685ed96a555c9aeec1217736bd83a96ebd409cc",
            "parentHash":"0x90926e0298d418181bd20c23b332451e35fd7d696b5dcdc5a3a0a6b715f4c717",
            "receiptsRoot":"0xd43aa19ecb03571d1b86d89d9bb980139d32f2f2ba59646cd5c1de9e80c68c90",
            "sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            "size":"0xdcc3",
            "stateRoot":"0x707875120a7103621fb4131df59904cda39de948dfda9084a1e3da44594d5404",
            "timestamp":"0x65f5f4c3",
            "transactionsRoot":"0x889a1c26dc42ba829dab552b779620feac231cde8a6c79af022bdc605c23a780",
            "uncles": ["0x123a1c26dc42ba829dab552b779620feac231cde8a6c79af022bdc605c23a780", "0x489a1c26dc42ba829dab552b779620feac231cde8a6c79af022bdc605c23a780"],
            "withdrawals":[
               {
                  "index":"0x24d80e6",
                  "validatorIndex":"0x8b2b6",
                  "address":"0x7cd1122e8e118b12ece8d25480dfeef230da17ff",
                  "amount":"0x1161f10"
               }
            ],
            "withdrawalsRoot":"0x360c33f20eeed5efbc7d08be46e58f8440af5db503e40908ef3d1eb314856ef7"
         }"#;

        let block = serde_json::from_str::<Block>(s).unwrap();
        assert_eq!(block.uncles.len(), 2);
        let serialized = serde_json::to_string(&block).unwrap();
        let block2 = serde_json::from_str::<Block>(&serialized).unwrap();
        assert_eq!(block, block2);
    }
}
