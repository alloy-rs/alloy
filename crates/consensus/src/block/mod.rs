//! Block-related consensus types.

mod header;
pub use header::{BlockHeader, Header};

#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) use header::serde_bincode_compat;

use crate::{transaction::SignerRecoverable, Transaction};
use alloc::vec::Vec;
use alloy_eips::{eip4895::Withdrawals, Encodable2718, Typed2718};
use alloy_primitives::{Address, Bytes, SignatureError, B256};
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
///
/// See p2p block encoding reference: <https://github.com/ethereum/devp2p/blob/master/caps/eth.md#block-encoding-and-validity>
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Deref)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Block<T, H = Header> {
    /// Block header.
    #[deref]
    pub header: H,
    /// Block body.
    pub body: BlockBody<T, H>,
}

impl<T, H> Block<T, H> {
    /// Creates a new block with the given header and body.
    pub const fn new(header: H, body: BlockBody<T, H>) -> Self {
        Self { header, body }
    }

    /// Creates a new empty uncle block.
    pub fn uncle(header: H) -> Self {
        Self { header, body: Default::default() }
    }

    /// Consumes the block and returns the header.
    pub fn into_header(self) -> H {
        self.header
    }

    /// Consumes the block and returns the body.
    pub fn into_body(self) -> BlockBody<T, H> {
        self.body
    }

    /// Converts the block's header type by applying a function to it.
    pub fn map_header<U>(self, mut f: impl FnMut(H) -> U) -> Block<T, U> {
        Block { header: f(self.header), body: self.body.map_ommers(f) }
    }

    /// Converts the block's header type by applying a fallible function to it.
    pub fn try_map_header<U, E>(
        self,
        mut f: impl FnMut(H) -> Result<U, E>,
    ) -> Result<Block<T, U>, E> {
        Ok(Block { header: f(self.header)?, body: self.body.try_map_ommers(f)? })
    }

    /// Converts the block's transaction type to the given alternative that is `From<T>`
    pub fn convert_transactions<U>(self) -> Block<U, H>
    where
        U: From<T>,
    {
        self.map_transactions(U::from)
    }

    /// Converts the block's transaction to the given alternative that is `TryFrom<T>`
    ///
    /// Returns the block with the new transaction type if all conversions were successful.
    pub fn try_convert_transactions<U>(self) -> Result<Block<U, H>, U::Error>
    where
        U: TryFrom<T>,
    {
        self.try_map_transactions(U::try_from)
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

    /// Returns the RLP encoded length of the block's header and body.
    pub fn rlp_length_for(header: &H, body: &BlockBody<T, H>) -> usize
    where
        H: Encodable,
        T: Encodable,
    {
        block_rlp::HelperRef {
            header,
            transactions: &body.transactions,
            ommers: &body.ommers,
            withdrawals: body.withdrawals.as_ref(),
        }
        .length()
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

impl<T, H> From<Block<T, H>> for BlockBody<T, H> {
    fn from(block: Block<T, H>) -> Self {
        block.into_body()
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T, H> arbitrary::Arbitrary<'a> for Block<T, H>
where
    T: arbitrary::Arbitrary<'a>,
    H: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self { header: u.arbitrary()?, body: u.arbitrary()? })
    }
}

/// A response to `GetBlockBodies`, containing bodies if any bodies were found.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[rlp(trailing)]
pub struct BlockBody<T, H = Header> {
    /// Transactions in this block.
    pub transactions: Vec<T>,
    /// Ommers/uncles header.
    pub ommers: Vec<H>,
    /// Block withdrawals.
    pub withdrawals: Option<Withdrawals>,
}

impl<T, H> Default for BlockBody<T, H> {
    fn default() -> Self {
        Self { transactions: Vec::new(), ommers: Vec::new(), withdrawals: None }
    }
}

impl<T, H> BlockBody<T, H> {
    /// Returns an iterator over all transactions.
    #[inline]
    pub fn transactions(&self) -> impl Iterator<Item = &T> + '_ {
        self.transactions.iter()
    }

    /// Create a [`Block`] from the body and its header.
    pub const fn into_block(self, header: H) -> Block<T, H> {
        Block { header, body: self }
    }

    /// Calculate the ommers root for the block body.
    pub fn calculate_ommers_root(&self) -> B256
    where
        H: Encodable,
    {
        crate::proofs::calculate_ommers_root(&self.ommers)
    }

    /// Calculate the withdrawals root for the block body, if withdrawals exist. If there are no
    /// withdrawals, this will return `None`.
    pub fn calculate_withdrawals_root(&self) -> Option<B256> {
        self.withdrawals.as_ref().map(|w| crate::proofs::calculate_withdrawals_root(w))
    }

    /// Converts the body's ommers type by applying a function to it.
    pub fn map_ommers<U>(self, f: impl FnMut(H) -> U) -> BlockBody<T, U> {
        BlockBody {
            transactions: self.transactions,
            ommers: self.ommers.into_iter().map(f).collect(),
            withdrawals: self.withdrawals,
        }
    }

    /// Converts the body's ommers type by applying a fallible function to it.
    pub fn try_map_ommers<U, E>(
        self,
        f: impl FnMut(H) -> Result<U, E>,
    ) -> Result<BlockBody<T, U>, E> {
        Ok(BlockBody {
            transactions: self.transactions,
            ommers: self.ommers.into_iter().map(f).collect::<Result<Vec<_>, _>>()?,
            withdrawals: self.withdrawals,
        })
    }
}

impl<T: Transaction, H> BlockBody<T, H> {
    /// Returns an iterator over all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes_iter(&self) -> impl Iterator<Item = &B256> + '_ {
        self.eip4844_transactions_iter().filter_map(|tx| tx.blob_versioned_hashes()).flatten()
    }
}

impl<T: Typed2718, H> BlockBody<T, H> {
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
        ommers: Vec<H>,
        withdrawals: Option<Withdrawals>,
    }

    #[derive(RlpEncodable)]
    #[rlp(trailing)]
    pub(crate) struct HelperRef<'a, T, H> {
        pub(crate) header: &'a H,
        pub(crate) transactions: &'a Vec<T>,
        pub(crate) ommers: &'a Vec<H>,
        pub(crate) withdrawals: Option<&'a Withdrawals>,
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

/// Represents the structure of a block.
pub trait BlockT<T: Transaction, H: BlockHeader> {
    /// Instantiates a new block with the given header and body.
    fn new(header: H, body: BlockBody<T, H>) -> Self;
    /// Returns reference to the block header.
    fn header(&self) -> &H;
    /// Consumes the block and returns the header.
    fn into_header(self) -> H;
    /// Returns reference to the block body.
    fn body(&self) -> &BlockBody<T, H>;
    /// Consumes the block and returns the body.
    fn into_body(self) -> BlockBody<T, H>;

    /// Splits the block into references of its header and body.
    fn split_ref(&self) -> (&H, &BlockBody<T, H>) {
        (self.header(), self.body())
    }

    /// Returns the rlp length of the block.
    fn rlp_length(&self) -> usize
    where
        T: Encodable,
        H: Encodable,
    {
        Block::<T, H>::rlp_length_for(self.header(), self.body())
    }

    /// Expensive operation that recovers transaction signers.
    fn recover_signers(&self) -> Result<Vec<Address>, SignatureError>
    where
        T: SignerRecoverable + Encodable2718,
        H: Encodable,
    {
        self.body().recover_signers()
    }
}

impl<T: Transaction, H: BlockHeader> BlockT<T, H> for Block<T, H> {
    fn new(header: H, body: BlockBody<T, H>) -> Self {
        Self::new(header, body)
    }

    fn header(&self) -> &H {
        &self.header
    }

    fn into_header(self) -> H {
        self.header
    }

    fn body(&self) -> &BlockBody<T, H> {
        &self.body
    }

    fn into_body(self) -> BlockBody<T, H> {
        self.body
    }
}

/// Represents the structure of a block body.
pub trait BlockBodyT<T: Transaction + Encodable2718, H: BlockHeader + Encodable> {
    /// Returns reference to transactions in the block.
    fn transactions(&self) -> &[T];

    /// Returns an iterator over the transactions in the block.
    fn transactions_iter(&self) -> impl Iterator<Item = &T> {
        self.transactions().iter()
    }

    /// Returns the number of the transactions in the block.
    fn transaction_count(&self) -> usize {
        self.transactions().len()
    }

    /// Consume the block body and return a [`Vec`] of transactions.
    fn into_transactions(self) -> Vec<T>;

    /// Returns `true` if the block body contains a transaction of the given type.
    fn contains_transaction_type(&self, tx_type: u8) -> bool {
        self.transactions_iter().any(|tx| tx.is_type(tx_type))
    }

    /// Clones the transactions in the block.
    ///
    /// This is a convenience function for `transactions().to_vec()`
    fn clone_transactions(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.transactions().to_vec()
    }

    /// Calculate the transaction root for the block body.
    fn calculate_tx_root(&self) -> B256 {
        crate::proofs::calculate_transaction_root(self.transactions())
    }

    /// Returns block withdrawals if any.
    fn withdrawals(&self) -> Option<&Withdrawals>;

    /// Calculate the withdrawals root for the block body.
    ///
    /// Returns `None` if there are no withdrawals in the block.
    fn calculate_withdrawals_root(&self) -> Option<B256> {
        self.withdrawals()
            .map(|withdrawals| crate::proofs::calculate_withdrawals_root(withdrawals.as_slice()))
    }

    /// Returns block ommers if any.
    fn ommers(&self) -> Option<&[H]>;

    /// Calculate the ommers root for the block body.
    ///
    /// Returns `None` if there are no ommers in the block.
    fn calculate_ommers_root(&self) -> Option<B256> {
        self.ommers().map(crate::proofs::calculate_ommers_root)
    }

    /// Calculates the total blob gas used by _all_ EIP-4844 transactions in the block.
    fn blob_gas_used(&self) -> u64 {
        self.transactions_iter().filter_map(|tx| tx.blob_gas_used()).sum()
    }

    /// Returns an iterator over all blob versioned hashes in the block body.
    fn blob_versioned_hashes_iter<'a>(&'a self) -> impl Iterator<Item = &'a B256> + 'a
    where
        H: 'a,
    {
        self.transactions_iter().filter_map(|tx| tx.blob_versioned_hashes()).flatten()
    }

    /// Returns an iterator over the encoded 2718 transactions.
    ///
    /// This is also known as `raw transactions`.
    ///
    /// See also [`Encodable2718`].
    #[doc(alias = "raw_transactions_iter")]
    fn encoded_2718_transactions_iter<'a>(&'a self) -> impl Iterator<Item = Vec<u8>> + 'a
    where
        H: 'a,
    {
        self.transactions_iter().map(|tx| tx.encoded_2718())
    }

    /// Returns a vector of encoded 2718 transactions.
    ///
    /// This is also known as `raw transactions`.
    ///
    /// See also [`Encodable2718`].
    #[doc(alias = "raw_transactions")]
    fn encoded_2718_transactions(&self) -> Vec<Bytes> {
        self.encoded_2718_transactions_iter().map(Into::into).collect()
    }

    /// Recover signer addresses for all transactions in the block body.
    fn recover_signers(&self) -> Result<Vec<Address>, SignatureError>
    where
        T: SignerRecoverable,
    {
        self.transactions_iter().map(|tx| tx.recover_signer()).collect()
    }

    /// Recover signer addresses for all transactions in the block body.
    ///
    /// Returns an error if some transaction's signature is invalid.
    fn try_recover_signers(&self) -> Result<Vec<Address>, SignatureError>
    where
        T: SignerRecoverable,
    {
        self.recover_signers()
    }

    /// Recover signer addresses for all transactions in the block body _without ensuring that the
    /// signature has a low `s` value_.
    ///
    /// Returns `None`, if some transaction's signature is invalid.
    fn recover_signers_unchecked(&self) -> Result<Vec<Address>, SignatureError>
    where
        T: SignerRecoverable,
    {
        self.transactions_iter().map(|tx| tx.recover_signer_unchecked()).collect()
    }

    /// Recover signer addresses for all transactions in the block body _without ensuring that the
    /// signature has a low `s` value_.
    ///
    /// Returns an error if some transaction's signature is invalid.
    fn try_recover_signers_unchecked(&self) -> Result<Vec<Address>, SignatureError>
    where
        T: SignerRecoverable,
    {
        self.recover_signers_unchecked()
    }
}

impl<T: Transaction + Encodable2718, H: BlockHeader + Encodable> BlockBodyT<T, H>
    for BlockBody<T, H>
{
    fn transactions(&self) -> &[T] {
        &self.transactions
    }

    fn into_transactions(self) -> Vec<T> {
        self.transactions
    }

    fn ommers(&self) -> Option<&[H]> {
        Some(&self.ommers)
    }

    fn withdrawals(&self) -> Option<&Withdrawals> {
        self.withdrawals.as_ref()
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T, H> arbitrary::Arbitrary<'a> for BlockBody<T, H>
where
    T: arbitrary::Arbitrary<'a>,
    H: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        // first generate up to 100 txs
        // first generate a reasonable amount of txs
        let transactions = (0..u.int_in_range(0..=100)?)
            .map(|_| T::arbitrary(u))
            .collect::<arbitrary::Result<Vec<_>>>()?;

        // then generate up to 2 ommers
        let ommers = (0..u.int_in_range(0..=1)?)
            .map(|_| H::arbitrary(u))
            .collect::<arbitrary::Result<Vec<_>>>()?;

        Ok(Self { transactions, ommers, withdrawals: u.arbitrary()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Signed, TxEnvelope, TxLegacy};

    #[test]
    fn can_convert_block() {
        let block: Block<Signed<TxLegacy>> = Block::default();
        let _: Block<TxEnvelope> = block.convert_transactions();
    }
}
