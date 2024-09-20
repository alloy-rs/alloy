//! Genesic Block Type

use crate::Header;
use alloy_eips::eip4895::Withdrawal;
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Block<T: Encodable + Decodable> {
    /// Block header.
    pub header: Header,
    /// Block body.
    #[rlp(flatten)]
    pub body: BlockBody<T>,
}

/// A block body.
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct BlockBody<T: Encodable + Decodable> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Vec<Withdrawal>>,
    // TODO: add request with rlp encoding support
}
