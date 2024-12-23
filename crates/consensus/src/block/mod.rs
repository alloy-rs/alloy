//! Block-related consensus types.

mod header;
pub use header::{BlockHeader, Header};

#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) use header::serde_bincode_compat;

use crate::{Transaction, Typed2718};
use alloc::vec::Vec;
use alloy_eips::eip4895::Withdrawals;
use alloy_primitives::B256;
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
///
/// See p2p block encoding reference: <https://github.com/ethereum/devp2p/blob/master/caps/eth.md#block-encoding-and-validity>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize, serde::Deserialize))]
pub struct Block<T, H = Header> {
    /// Block header.
    pub header: H,
    /// Block body.
    pub body: BlockBody<T>,
}

impl<T, H> Block<T, H> {
    /// Creates a new empty uncle block.
    pub fn uncle(header: H) -> Self {
        Self { header, body: Default::default() }
    }

    /// Consumes the block and returns the header.
    pub fn into_header(self) -> H {
        self.header
    }

    /// Consumes the block and returns the body.
    pub fn into_body(self) -> BlockBody<T> {
        self.body
    }

    /// Converts the block's header type by applying a function to it.
    pub fn map_header<U>(self, f: impl FnOnce(H) -> U) -> Block<T, U> {
        Block { header: f(self.header), body: self.body }
    }

    /// Converts the block's header type by applying a fallible function to it.
    pub fn try_map_header<U, E>(self, f: impl FnOnce(H) -> Result<U, E>) -> Result<Block<T, U>, E> {
        Ok(Block { header: f(self.header)?, body: self.body })
    }

    /// Converts the block's transaction type by applying a function to each transaction.
    ///
    /// Returns the block with the new transaction type.
    pub fn map_transactions<U>(self, f: impl FnMut(T) -> U) -> Block<U, H> {
        Block {
            header: self.header,
            body: BlockBody {
                transactions: self.body.transactions.into_iter().map(f).collect(),
                ommers: self.body.ommers,
                withdrawals: self.body.withdrawals,
            },
        }
    }

    /// Converts the block's transaction type by applying a fallible function to each transaction.
    ///
    /// Returns the block with the new transaction type if all transactions were successfully.
    pub fn try_map_transactions<U, E>(
        self,
        f: impl FnMut(T) -> Result<U, E>,
    ) -> Result<Block<U, H>, E> {
        Ok(Block {
            header: self.header,
            body: BlockBody {
                transactions: self
                    .body
                    .transactions
                    .into_iter()
                    .map(f)
                    .collect::<Result<_, _>>()?,
                ommers: self.body.ommers,
                withdrawals: self.body.withdrawals,
            },
        })
    }
}

impl<T, H> Default for Block<T, H>
where
    H: Default,
{
    fn default() -> Self {
        Self { header: Default::default(), body: Default::default() }
    }
}

impl<T, H> From<Block<T, H>> for BlockBody<T> {
    fn from(block: Block<T, H>) -> Self {
        block.into_body()
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T> arbitrary::Arbitrary<'a> for Block<T>
where
    T: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        // first generate a reasonable amount of txs
        let transactions = (0..u.int_in_range(0..=100)?)
            .map(|_| T::arbitrary(u))
            .collect::<arbitrary::Result<Vec<_>>>()?;

        // then generate up to 2 ommers
        let ommers = (0..u.int_in_range(0..=1)?)
            .map(|_| Header::arbitrary(u))
            .collect::<arbitrary::Result<Vec<_>>>()?;

        Ok(Self {
            header: u.arbitrary()?,
            body: BlockBody { transactions, ommers, withdrawals: u.arbitrary()? },
        })
    }
}

/// A response to `GetBlockBodies`, containing bodies if any bodies were found.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize, serde::Deserialize))]
#[rlp(trailing)]
pub struct BlockBody<T> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Withdrawals>,
}

impl<T> Default for BlockBody<T> {
    fn default() -> Self {
        Self { transactions: Vec::new(), ommers: Vec::new(), withdrawals: None }
    }
}

impl<T> BlockBody<T> {
    /// Returns an iterator over all transactions.
    #[inline]
    pub fn transactions(&self) -> impl Iterator<Item = &T> + '_ {
        self.transactions.iter()
    }

    /// Create a [`Block`] from the body and its header.
    pub const fn into_block(self, header: Header) -> Block<T> {
        Block { header, body: self }
    }
}

impl<T> BlockBody<T> {
    /// Calculate the ommers root for the block body.
    pub fn calculate_ommers_root(&self) -> B256 {
        crate::proofs::calculate_ommers_root(&self.ommers)
    }

    /// Calculate the withdrawals root for the block body, if withdrawals exist. If there are no
    /// withdrawals, this will return `None`.
    pub fn calculate_withdrawals_root(&self) -> Option<B256> {
        self.withdrawals.as_ref().map(|w| crate::proofs::calculate_withdrawals_root(w))
    }
}

impl<T: Transaction> BlockBody<T> {
    /// Returns an iterator over all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes_iter(&self) -> impl Iterator<Item = &B256> + '_ {
        self.eip4844_transactions_iter().filter_map(|tx| tx.blob_versioned_hashes()).flatten()
    }
}

impl<T: Typed2718> BlockBody<T> {
    /// Returns whether or not the block body contains any blob transactions.
    #[inline]
    pub fn has_eip4844_transactions(&self) -> bool {
        self.transactions.iter().any(|tx| tx.is_eip4844())
    }

    /// Returns whether or not the block body contains any EIP-7702 transactions.
    #[inline]
    pub fn has_eip7702_transactions(&self) -> bool {
        self.transactions.iter().any(|tx| tx.is_eip7702())
    }

    /// Returns an iterator over all blob transactions of the block.
    #[inline]
    pub fn eip4844_transactions_iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.transactions.iter().filter(|tx| tx.is_eip4844())
    }
}

/// We need to implement RLP traits manually because we currently don't have a way to flatten
/// [`BlockBody`] into [`Block`].
mod block_rlp {
    use super::*;

    #[derive(RlpDecodable)]
    #[rlp(trailing)]
    struct Helper<T, H> {
        header: H,
        transactions: Vec<T>,
        ommers: Vec<Header>,
        withdrawals: Option<Withdrawals>,
    }

    #[derive(RlpEncodable)]
    #[rlp(trailing)]
    struct HelperRef<'a, T, H> {
        header: &'a H,
        transactions: &'a Vec<T>,
        ommers: &'a Vec<Header>,
        withdrawals: Option<&'a Withdrawals>,
    }

    impl<'a, T, H> From<&'a Block<T, H>> for HelperRef<'a, T, H> {
        fn from(block: &'a Block<T, H>) -> Self {
            let Block { header, body: BlockBody { transactions, ommers, withdrawals } } = block;
            Self { header, transactions, ommers, withdrawals: withdrawals.as_ref() }
        }
    }

    impl<T: Encodable, H: Encodable> Encodable for Block<T, H> {
        fn encode(&self, out: &mut dyn alloy_rlp::bytes::BufMut) {
            let helper: HelperRef<'_, T, H> = self.into();
            helper.encode(out)
        }

        fn length(&self) -> usize {
            let helper: HelperRef<'_, T, H> = self.into();
            helper.length()
        }
    }

    impl<T: Decodable, H: Decodable> Decodable for Block<T, H> {
        fn decode(b: &mut &[u8]) -> alloy_rlp::Result<Self> {
            let Helper { header, transactions, ommers, withdrawals } = Helper::decode(b)?;
            Ok(Self { header, body: BlockBody { transactions, ommers, withdrawals } })
        }
    }
}
