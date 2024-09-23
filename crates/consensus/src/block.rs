//! Genesic Block Type

use crate::{Header, Requests};
use alloc::vec::Vec;
use alloy_eips::eip4895::Withdrawal;
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
///
/// See p2p block encoding reference: <https://github.com/ethereum/devp2p/blob/master/caps/eth.md#block-encoding-and-validity>
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Block<T: Encodable + Decodable> {
    /// Block header.
    pub header: Header,
    /// Block body.
    #[rlp(flatten)]
    pub body: BlockBody<T>,
}

/// A response to `GetBlockBodies`, containing bodies if any bodies were found.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct BlockBody<T: Encodable + Decodable> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Block requests
    pub requests: Option<Requests>,
}
