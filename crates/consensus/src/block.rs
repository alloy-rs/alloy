//! An Ethereum block.

use crate::{Header, Withdrawals, Requests, Signed};
use alloy_primitives::{B256, Address};
use alloy_rlp::{RlpDecodable, RlpEncodable};

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deref, RlpEncodable, RlpDecodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[rlp(trailing)]
pub struct Block<T> {
    /// Block header.
    #[deref]
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

impl Block {
    /// Calculate the header hash and seal the block so that it can't be changed.
    pub fn seal_slow(self) -> SealedBlock {
        SealedBlock {
            header: self.header.seal_slow(),
            body: self.body,
            ommers: self.ommers,
            withdrawals: self.withdrawals,
            requests: self.requests,
        }
    }

    /// Seal the block with a known hash.
    ///
    /// WARNING: This method does not perform validation whether the hash is correct.
    pub fn seal(self, hash: B256) -> SealedBlock {
        SealedBlock {
            header: self.header.seal(hash),
            body: self.body,
            ommers: self.ommers,
            withdrawals: self.withdrawals,
            requests: self.requests,
        }
    }

    /// Expensive operation that recovers transaction signer. See [`SealedBlockWithSenders`].
    pub fn senders(&self) -> Option<Vec<Address>> {
        Signed::recover_signers(&self.body, self.body.len())
    }

    /// Transform into a [`BlockWithSenders`].
    ///
    /// # Panics
    ///
    /// If the number of senders does not match the number of transactions in the block
    /// and the signer recovery for one of the transactions fails.
    ///
    /// Note: this is expected to be called with blocks read from disk.
    #[track_caller]
    pub fn with_senders_unchecked(self, senders: Vec<Address>) -> BlockWithSenders {
        self.try_with_senders_unchecked(senders).expect("stored block is valid")
    }

    /// Transform into a [`BlockWithSenders`] using the given senders.
    ///
    /// If the number of senders does not match the number of transactions in the block, this falls
    /// back to manually recovery, but _without ensuring that the signature has a low `s` value_.
    /// See also [`TransactionSigned::recover_signer_unchecked`]
    ///
    /// Returns an error if a signature is invalid.
    #[track_caller]
    pub fn try_with_senders_unchecked(
        self,
        senders: Vec<Address>,
    ) -> Result<BlockWithSenders, Self> {
        let senders = if self.body.len() == senders.len() {
            senders
        } else {
            let Some(senders) =
                TransactionSigned::recover_signers_unchecked(&self.body, self.body.len())
            else {
                return Err(self)
            };
            senders
        };

        Ok(BlockWithSenders { block: self, senders })
    }

    /// **Expensive**. Transform into a [`BlockWithSenders`] by recovering senders in the contained
    /// transactions.
    ///
    /// Returns `None` if a transaction is invalid.
    pub fn with_recovered_senders(self) -> Option<BlockWithSenders> {
        let senders = self.senders()?;
        Some(BlockWithSenders { block: self, senders })
    }

    /// Returns whether or not the block contains any blob transactions.
    #[inline]
    pub fn has_blob_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.is_eip4844())
    }

    /// Returns whether or not the block contains any EIP-7702 transactions.
    #[inline]
    pub fn has_eip7702_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.is_eip7702())
    }

    /// Returns an iterator over all blob transactions of the block
    #[inline]
    pub fn blob_transactions_iter(&self) -> impl Iterator<Item = &Signed> + '_ {
        self.body.iter().filter(|tx| tx.is_eip4844())
    }

    /// Returns only the blob transactions, if any, from the block body.
    #[inline]
    pub fn blob_transactions(&self) -> Vec<&Signed> {
        self.blob_transactions_iter().collect()
    }

    /// Returns an iterator over all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes_iter(&self) -> impl Iterator<Item = &B256> + '_ {
        self.blob_transactions_iter()
            .filter_map(|tx| tx.as_eip4844().map(|blob_tx| &blob_tx.blob_versioned_hashes))
            .flatten()
    }

    /// Returns all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes(&self) -> Vec<&B256> {
        self.blob_versioned_hashes_iter().collect()
    }

    /// Calculates a heuristic for the in-memory size of the [`Block`].
    #[inline]
    pub fn size(&self) -> usize {
        self.header.size() +
            // take into account capacity
            self.body.iter().map(Signed::size).sum::<usize>() + self.body.capacity() * core::mem::size_of::<TransactionSigned>() +
            self.ommers.iter().map(Header::size).sum::<usize>() + self.ommers.capacity() * core::mem::size_of::<Header>() +
            self.withdrawals.as_ref().map_or(core::mem::size_of::<Option<Withdrawals>>(), Withdrawals::total_size)
    }
}


/// Sealed block with senders recovered from transactions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deref, DerefMut)]
pub struct BlockWithSenders {
    /// Block
    #[deref]
    #[deref_mut]
    pub block: Block,
    /// List of senders that match the transactions in the block
    pub senders: Vec<Address>,
}

impl BlockWithSenders {
    /// New block with senders. Return none if len of tx and senders does not match
    pub fn new(block: Block, senders: Vec<Address>) -> Option<Self> {
        (block.body.len() == senders.len()).then_some(Self { block, senders })
    }

    /// Seal the block with a known hash.
    ///
    /// WARNING: This method does not perform validation whether the hash is correct.
    #[inline]
    pub fn seal(self, hash: B256) -> SealedBlockWithSenders {
        let Self { block, senders } = self;
        SealedBlockWithSenders { block: block.seal(hash), senders }
    }

    /// Calculate the header hash and seal the block with senders so that it can't be changed.
    #[inline]
    pub fn seal_slow(self) -> SealedBlockWithSenders {
        SealedBlockWithSenders { block: self.block.seal_slow(), senders: self.senders }
    }

    /// Split Structure to its components
    #[inline]
    pub fn into_components(self) -> (Block, Vec<Address>) {
        (self.block, self.senders)
    }

    /// Returns an iterator over all transactions in the block.
    #[inline]
    pub fn transactions(&self) -> impl Iterator<Item = &Signed> + '_ {
        self.block.body.iter()
    }

    /// Returns an iterator over all transactions and their sender.
    #[inline]
    pub fn transactions_with_sender(
        &self,
    ) -> impl Iterator<Item = (&Address, &Signed)> + '_ {
        self.senders.iter().zip(self.block.body.iter())
    }

    /// Consumes the block and returns the transactions of the block.
    #[inline]
    pub fn into_transactions(self) -> Vec<Signed> {
        self.block.body
    }

    /// Returns an iterator over all transactions in the chain.
    #[inline]
    pub fn into_transactions_ecrecovered(
        self,
    ) -> impl Iterator<Item = TransactionSignedEcRecovered> {
        self.block.body.into_iter().zip(self.senders).map(|(tx, sender)| tx.with_signer(sender))
    }
}

/// Sealed Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(any(test, feature = "reth-codec"), reth_codecs::add_arbitrary_tests(rlp, 32))]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    RlpEncodable,
    RlpDecodable,
)]
#[rlp(trailing)]
pub struct SealedBlock {
    /// Locked block header.
    #[deref]
    #[deref_mut]
    pub header: SealedHeader,
    /// Transactions with signatures.
    pub body: Vec<TransactionSigned>,
    /// Ommer/uncle headers
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Withdrawals>,
    /// Block requests.
    pub requests: Option<Requests>,
}

impl SealedBlock {
    /// Create a new sealed block instance using the sealed header and block body.
    #[inline]
    pub fn new(header: SealedHeader, body: BlockBody) -> Self {
        let BlockBody { transactions, ommers, withdrawals, requests } = body;
        Self { header, body: transactions, ommers, withdrawals, requests }
    }

    /// Header hash.
    #[inline]
    pub const fn hash(&self) -> B256 {
        self.header.hash()
    }

    /// Splits the sealed block into underlying components
    #[inline]
    pub fn split(self) -> (SealedHeader, Vec<TransactionSigned>, Vec<Header>) {
        (self.header, self.body, self.ommers)
    }

    /// Splits the [`BlockBody`] and [`SealedHeader`] into separate components
    #[inline]
    pub fn split_header_body(self) -> (SealedHeader, BlockBody) {
        (
            self.header,
            BlockBody {
                transactions: self.body,
                ommers: self.ommers,
                withdrawals: self.withdrawals,
                requests: self.requests,
            },
        )
    }

    /// Returns an iterator over all blob transactions of the block
    #[inline]
    pub fn blob_transactions_iter(&self) -> impl Iterator<Item = &TransactionSigned> + '_ {
        self.body.iter().filter(|tx| tx.is_eip4844())
    }

    /// Returns only the blob transactions, if any, from the block body.
    #[inline]
    pub fn blob_transactions(&self) -> Vec<&TransactionSigned> {
        self.blob_transactions_iter().collect()
    }

    /// Returns an iterator over all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes_iter(&self) -> impl Iterator<Item = &B256> + '_ {
        self.blob_transactions_iter()
            .filter_map(|tx| tx.as_eip4844().map(|blob_tx| &blob_tx.blob_versioned_hashes))
            .flatten()
    }

    /// Returns all blob versioned hashes from the block body.
    #[inline]
    pub fn blob_versioned_hashes(&self) -> Vec<&B256> {
        self.blob_versioned_hashes_iter().collect()
    }

    /// Expensive operation that recovers transaction signer. See [`SealedBlockWithSenders`].
    pub fn senders(&self) -> Option<Vec<Address>> {
        TransactionSigned::recover_signers(&self.body, self.body.len())
    }

    /// Seal sealed block with recovered transaction senders.
    pub fn seal_with_senders(self) -> Option<SealedBlockWithSenders> {
        self.try_seal_with_senders().ok()
    }

    /// Seal sealed block with recovered transaction senders.
    pub fn try_seal_with_senders(self) -> Result<SealedBlockWithSenders, Self> {
        match self.senders() {
            Some(senders) => Ok(SealedBlockWithSenders { block: self, senders }),
            None => Err(self),
        }
    }

    /// Transform into a [`SealedBlockWithSenders`].
    ///
    /// # Panics
    ///
    /// If the number of senders does not match the number of transactions in the block
    /// and the signer recovery for one of the transactions fails.
    #[track_caller]
    pub fn with_senders_unchecked(self, senders: Vec<Address>) -> SealedBlockWithSenders {
        self.try_with_senders_unchecked(senders).expect("stored block is valid")
    }

    /// Transform into a [`SealedBlockWithSenders`] using the given senders.
    ///
    /// If the number of senders does not match the number of transactions in the block, this falls
    /// back to manually recovery, but _without ensuring that the signature has a low `s` value_.
    /// See also [`TransactionSigned::recover_signer_unchecked`]
    ///
    /// Returns an error if a signature is invalid.
    #[track_caller]
    pub fn try_with_senders_unchecked(
        self,
        senders: Vec<Address>,
    ) -> Result<SealedBlockWithSenders, Self> {
        let senders = if self.body.len() == senders.len() {
            senders
        } else {
            let Some(senders) =
                TransactionSigned::recover_signers_unchecked(&self.body, self.body.len())
            else {
                return Err(self)
            };
            senders
        };

        Ok(SealedBlockWithSenders { block: self, senders })
    }

    /// Unseal the block
    pub fn unseal(self) -> Block {
        Block {
            header: self.header.unseal(),
            body: self.body,
            ommers: self.ommers,
            withdrawals: self.withdrawals,
            requests: self.requests,
        }
    }

    /// Calculates a heuristic for the in-memory size of the [`SealedBlock`].
    #[inline]
    pub fn size(&self) -> usize {
        self.header.size() +
            // take into account capacity
            self.body.iter().map(TransactionSigned::size).sum::<usize>() + self.body.capacity() * core::mem::size_of::<TransactionSigned>() +
            self.ommers.iter().map(Header::size).sum::<usize>() + self.ommers.capacity() * core::mem::size_of::<Header>() +
            self.withdrawals.as_ref().map_or(core::mem::size_of::<Option<Withdrawals>>(), Withdrawals::total_size)
    }

    /// Calculates the total gas used by blob transactions in the sealed block.
    pub fn blob_gas_used(&self) -> u64 {
        self.blob_transactions().iter().filter_map(|tx| tx.blob_gas_used()).sum()
    }

    /// Returns whether or not the block contains any blob transactions.
    #[inline]
    pub fn has_blob_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.is_eip4844())
    }

    /// Returns whether or not the block contains any eip-7702 transactions.
    #[inline]
    pub fn has_eip7702_transactions(&self) -> bool {
        self.body.iter().any(|tx| tx.is_eip7702())
    }

    /// Ensures that the transaction root in the block header is valid.
    ///
    /// The transaction root is the Keccak 256-bit hash of the root node of the trie structure
    /// populated with each transaction in the transactions list portion of the block.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the calculated transaction root matches the one stored in the header,
    /// indicating that the transactions in the block are correctly represented in the trie.
    ///
    /// Returns `Err(error)` if the transaction root validation fails, providing a `GotExpected`
    /// error containing the calculated and expected roots.
    pub fn ensure_transaction_root_valid(&self) -> Result<(), GotExpected<B256>> {
        let calculated_root = crate::proofs::calculate_transaction_root(&self.body);

        if self.header.transactions_root != calculated_root {
            return Err(GotExpected {
                got: calculated_root,
                expected: self.header.transactions_root,
            })
        }

        Ok(())
    }

    /// Returns a vector of transactions RLP encoded with [`TransactionSigned::encode_enveloped`].
    pub fn raw_transactions(&self) -> Vec<Bytes> {
        self.body.iter().map(|tx| tx.envelope_encoded()).collect()
    }
}

impl From<SealedBlock> for Block {
    fn from(block: SealedBlock) -> Self {
        block.unseal()
    }
}

/// Sealed block with senders recovered from transactions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct SealedBlockWithSenders {
    /// Sealed block
    #[deref]
    #[deref_mut]
    pub block: SealedBlock,
    /// List of senders that match transactions from block.
    pub senders: Vec<Address>,
}

impl SealedBlockWithSenders {
    /// New sealed block with sender. Return none if len of tx and senders does not match
    pub fn new(block: SealedBlock, senders: Vec<Address>) -> Option<Self> {
        (block.body.len() == senders.len()).then_some(Self { block, senders })
    }

    /// Split Structure to its components
    #[inline]
    pub fn into_components(self) -> (SealedBlock, Vec<Address>) {
        (self.block, self.senders)
    }

    /// Returns the unsealed [`BlockWithSenders`]
    #[inline]
    pub fn unseal(self) -> BlockWithSenders {
        let Self { block, senders } = self;
        BlockWithSenders { block: block.unseal(), senders }
    }

    /// Returns an iterator over all transactions in the block.
    #[inline]
    pub fn transactions(&self) -> impl Iterator<Item = &Signed> + '_ {
        self.block.body.iter()
    }

    /// Returns an iterator over all transactions and their sender.
    #[inline]
    pub fn transactions_with_sender(
        &self,
    ) -> impl Iterator<Item = (&Address, &Signed)> + '_ {
        self.senders.iter().zip(self.block.body.iter())
    }

    /// Consumes the block and returns the transactions of the block.
    #[inline]
    pub fn into_transactions(self) -> Vec<Signed> {
        self.block.body
    }

    /// Returns an iterator over all transactions in the chain.
    #[inline]
    pub fn into_transactions_ecrecovered(
        self,
    ) -> impl Iterator<Item = TransactionSignedEcRecovered> {
        self.block.body.into_iter().zip(self.senders).map(|(tx, sender)| tx.with_signer(sender))
    }
}
