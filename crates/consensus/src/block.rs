//! Genesic Block Type

use crate::{Header, Requests};
use alloc::vec::Vec;
use alloy_eips::{
    eip2718::{Decodable2718, Encodable2718},
    eip4895::Withdrawal,
};
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Block<T: Encodable + Decodable + Encodable2718 + Decodable2718> {
    /// Block header.
    pub header: Header,
    /// Block body.
    #[rlp(flatten)]
    pub body: BlockBody<T>,
}

/// A block body.
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct BlockBody<T: Encodable + Decodable + Encodable2718 + Decodable2718> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Block requests
    pub requests: Option<Requests>,
}
