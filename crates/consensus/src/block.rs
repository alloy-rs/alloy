//! Block Type

use crate::Header;
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
pub struct Block<T> {
    /// Block header.
    pub header: Header,
    /// Block body.
    pub body: BlockBody<T>,
}

/// A response to `GetBlockBodies`, containing bodies if any bodies were found.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct BlockBody<T> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Withdrawals>,
}

/// We need to implement RLP traits manually because we currently don't have a way to flatten
/// [`BlockBody`] into [`Block`].
mod block_rlp {
    use super::*;

    #[derive(RlpDecodable)]
    #[rlp(trailing)]
    struct Helper<T> {
        header: Header,
        transactions: Vec<T>,
        ommers: Vec<Header>,
        withdrawals: Option<Withdrawals>,
    }

    #[derive(RlpEncodable)]
    #[rlp(trailing)]
    struct HelperRef<'a, T> {
        header: &'a Header,
        transactions: &'a Vec<T>,
        ommers: &'a Vec<Header>,
        withdrawals: Option<&'a Withdrawals>,
    }

    impl<'a, T> From<&'a Block<T>> for HelperRef<'a, T> {
        fn from(block: &'a Block<T>) -> Self {
            let Block { header, body: BlockBody { transactions, ommers, withdrawals } } = block;
            Self { header, transactions, ommers, withdrawals: withdrawals.as_ref() }
        }
    }

    impl<T: Encodable> Encodable for Block<T> {
        fn encode(&self, out: &mut dyn alloy_rlp::bytes::BufMut) {
            let helper: HelperRef<'_, T> = self.into();
            helper.encode(out)
        }

        fn length(&self) -> usize {
            let helper: HelperRef<'_, T> = self.into();
            helper.length()
        }
    }

    impl<T: Decodable> Decodable for Block<T> {
        fn decode(b: &mut &[u8]) -> alloy_rlp::Result<Self> {
            let Helper { header, transactions, ommers, withdrawals } = Helper::decode(b)?;
            Ok(Self { header, body: BlockBody { transactions, ommers, withdrawals } })
        }
    }
}
