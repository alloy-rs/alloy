//! Block-related consensus types.

mod header;
pub use header::{BlockHeader, BlockWithParent, Header};
mod any;
pub use any::AnyHeader;

#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) use header::serde_bincode_compat;

use alloc::vec::Vec;
use alloy_eips::eip4895::Withdrawals;
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
///
/// See p2p block encoding reference: <https://github.com/ethereum/devp2p/blob/master/caps/eth.md#block-encoding-and-validity>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
}

/// A response to `GetBlockBodies`, containing bodies if any bodies were found.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
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
