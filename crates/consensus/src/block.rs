//! An Ethereum block.

use crate::{Header, TxEnvelope, Transaction, TxType, Withdrawals, Requests, Signed};
use alloy_primitives::{B256, Sealable, Address};
use alloy_rlp::{RlpDecodable, RlpEncodable};

#[cfg(feature = "k256")]
use crate::SignableTransaction;
#[cfg(feature = "k256")]
use alloy_primitives::Signature;

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[rlp(trailing)]
pub struct Block<T> {
    /// Block header.
    pub header: Header,
    /// Transactions in this block.
    pub body: Vec<Signed<T>>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Withdrawals>,
    /// Block requests.
    pub requests: Option<Requests>,
}

impl<T> Sealable for Block<T> {
    fn hash_slow(&self) -> B256 {
        self.header.hash_slow()
    }
}

impl<T> Block<T>
where
    T: Transaction + std::fmt::Debug,
{
    /// Returns whether or not the block contains any blob transactions.
    #[inline]
    pub fn has_blob_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.tx().ty() == TxType::Eip4844 as u8)
    }

    /// Returns whether or not the block contains any EIP-7702 transactions.
    #[inline]
    pub fn has_eip7702_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.tx().ty() == TxType::Eip7702 as u8)
    }

    /// Returns an iterator over all blob transactions of the block
    #[inline]
    pub fn blob_transactions_iter(&self) -> impl Iterator<Item = &Signed<T>> + '_ {
        self.body.iter().filter(|tx| tx.tx().ty() == TxType::Eip4844 as u8)
    }

    /// Returns only the blob transactions, if any, from the block body.
    #[inline]
    pub fn blob_transactions(&self) -> Vec<&Signed<T>> {
        self.blob_transactions_iter().collect()
    }

    /// Returns an iterator over all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes_iter(&self) -> impl Iterator<Item = &B256> + '_ {
        self.blob_transactions_iter()
            .filter_map(|tx| tx.tx().blob_versioned_hashes())
            .flatten()
    }

    /// Returns all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes(&self) -> Vec<&B256> {
        self.blob_versioned_hashes_iter().collect()
    }
}

impl Block<TxEnvelope> {
    /// Calculates a heuristic for the in-memory size of the [`Block`].
    #[inline]
    pub fn size(&self) -> usize {
        self.header.size() +
            self.body.iter().map(|tx| tx.size()).sum::<usize>() +
            self.ommers.iter().map(Header::size).sum::<usize>() + self.ommers.capacity() * core::mem::size_of::<Header>() +
            self.withdrawals.as_ref().map_or(core::mem::size_of::<Option<Withdrawals>>(), Withdrawals::total_size)
    }
}

#[cfg(feature = "k256")]
impl<T> Block<T>
where
    T: Transaction + Sealable + SignableTransaction<Signature> + std::fmt::Debug,
{
    /// Expensive operation that recovers transaction signer. See [`SealedBlockWithSenders`].
    pub fn senders(&self) -> Option<Vec<Address>> {
        let mut senders = Vec::with_capacity(self.body.len());
        for tx in &self.body {
            match Signed::recover_signer(tx) {
                Ok(sender) => senders.push(sender),
                Err(_) => return None,
            }
        }
        Some(senders)
    }
}
