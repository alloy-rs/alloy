//! Block-related consensus types.

mod header;
pub use header::{BlockHeader, GasLimitMismatch, Header};

mod traits;
pub use traits::EthBlock;

mod meta;
pub use meta::{HeaderInfo, HeaderRoots};

#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) use header::serde_bincode_compat;

use crate::Transaction;
use alloc::vec::Vec;
use alloy_eips::{eip2718::WithEncoded, eip4895::Withdrawals, Encodable2718, Typed2718};
use alloy_primitives::{keccak256, Sealable, Sealed, B256};
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
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
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

    /// Converts the transactions in the block's body to `WithEncoded<T>` by encoding them via
    /// [`Encodable2718`]
    pub fn into_with_encoded2718(self) -> Block<WithEncoded<T>, H>
    where
        T: Encodable2718,
    {
        self.map_transactions(|tx| tx.into_encoded())
    }

    /// Replaces the header of the block.
    ///
    /// Note: This method only replaces the main block header. If you need to transform
    /// the ommer headers as well, use [`map_header`](Self::map_header) instead.
    pub fn with_header(mut self, header: H) -> Self {
        self.header = header;
        self
    }

    /// Encodes the [`Block`] given header and block body.
    ///
    /// Returns the rlp encoded block.
    ///
    /// This is equivalent to `block.encode`.
    pub fn rlp_encoded_from_parts(header: &H, body: &BlockBody<T, H>) -> Vec<u8>
    where
        H: Encodable,
        T: Encodable,
    {
        let helper = block_rlp::HelperRef::from_parts(header, body);
        let mut buf = Vec::with_capacity(helper.length());
        helper.encode(&mut buf);
        buf
    }

    /// Encodes the [`Block`] given header and block body
    ///
    /// This is equivalent to `block.encode`.
    pub fn rlp_encode_from_parts(
        header: &H,
        body: &BlockBody<T, H>,
        out: &mut dyn alloy_rlp::bytes::BufMut,
    ) where
        H: Encodable,
        T: Encodable,
    {
        block_rlp::HelperRef::from_parts(header, body).encode(out)
    }

    /// Returns the RLP encoded length of the block's header and body.
    pub fn rlp_length_for(header: &H, body: &BlockBody<T, H>) -> usize
    where
        H: Encodable,
        T: Encodable,
    {
        block_rlp::HelperRef::from_parts(header, body).length()
    }
}

impl<T: Encodable2718> Block<T, Header> {
    /// Creates a new block from a header and an iterator of transactions.
    ///
    /// Computes and sets the `transactions_root` on the header automatically.
    /// `ommers_hash` is set to [`EMPTY_OMMER_ROOT_HASH`](crate::EMPTY_OMMER_ROOT_HASH).
    ///
    /// This updates no other header fields, creates a body without withdrawals, and does not
    /// calculate the receipts root, validate transactions, or seal the header.
    pub fn from_transactions(
        mut header: Header,
        transactions: impl IntoIterator<Item = T>,
    ) -> Self {
        let transactions: Vec<T> = transactions.into_iter().collect();
        header.transactions_root = crate::proofs::calculate_transaction_root(&transactions);
        header.ommers_hash = crate::EMPTY_OMMER_ROOT_HASH;
        Self::new(header, BlockBody { transactions, ommers: Vec::new(), withdrawals: None })
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
///
/// Note: `Decodable` is implemented manually rather than derived, so that the transaction list is
/// pre-sized, see [`decode_transactions`].
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[rlp(trailing(no_gaps))]
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

    /// Returns an iterator over the hashes of the ommers in the block body.
    pub fn ommers_hashes(&self) -> impl Iterator<Item = B256> + '_
    where
        H: Sealable,
    {
        self.ommers.iter().map(|h| h.hash_slow())
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

/// Number of transactions [`decode_transactions`] decodes before it first estimates how many more
/// are coming.
///
/// Large enough to smooth out the size of an individual transaction, small enough that the `Vec`
/// is still tiny if the estimate turns out to be wrong.
const TX_CAPACITY_SAMPLE_SIZE: usize = 16;

/// Upper bound on the transaction capacity [`decode_transactions`] reserves, expressed as a
/// multiple of the remaining encoded bytes.
///
/// Without a bound, a list of very small items would let a peer make us reserve
/// `item_count * size_of::<T>()` bytes. Capping the reservation relative to the encoded size keeps
/// the memory amplification in the same ballpark as a legitimate block of the same wire size.
const MAX_TX_PREALLOC_RATIO: usize = 8;

/// Decodes an RLP list of transactions, pre-sizing the returned [`Vec`].
///
/// This is equivalent to `Vec::<T>::decode`, including the errors it returns, but avoids the
/// repeated reallocation that growing a `Vec` from empty incurs for blocks that carry thousands
/// of transactions: with no capacity hint, a 10k transaction block runs through the whole doubling
/// sequence and memmoves megabytes of `T` on the way.
///
/// Instead, once the vector is full, the average encoded transaction size of everything decoded so
/// far is used to extrapolate how many transactions are left, and that many are reserved in one
/// go. A wrong estimate is harmless: too low and the next exhausted capacity re-estimates from a
/// larger and therefore better sample, too high and the reservation is capped relative to the
/// number of bytes that are actually left.
///
/// # Examples
///
/// ```
/// use alloy_consensus::{decode_transactions, TxEnvelope};
///
/// # fn main() -> Result<(), alloy_rlp::Error> {
/// let encoded = alloy_rlp::encode(&Vec::<TxEnvelope>::new());
/// let txs: Vec<TxEnvelope> = decode_transactions(&mut encoded.as_slice())?;
/// assert!(txs.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn decode_transactions<T: Decodable>(buf: &mut &[u8]) -> alloy_rlp::Result<Vec<T>> {
    let mut payload = alloy_rlp::Header::decode_bytes(buf, true)?;
    let payload_length = payload.len();

    let mut transactions = Vec::new();
    while !payload.is_empty() {
        if transactions.len() >= TX_CAPACITY_SAMPLE_SIZE
            && transactions.len() == transactions.capacity()
        {
            // Average encoded size of the transactions decoded so far, at least 1 byte.
            let average = (payload_length - payload.len()) / transactions.len();
            let estimate = payload.len() / average.max(1) + 1;
            // Never reserve more than `MAX_TX_PREALLOC_RATIO` times the bytes that are left.
            let max = MAX_TX_PREALLOC_RATIO.saturating_mul(payload.len())
                / core::mem::size_of::<T>().max(1);
            transactions.reserve(estimate.min(max));
        }

        transactions.push(T::decode(&mut payload)?);
    }

    Ok(transactions)
}

/// We need to implement RLP traits manually because we currently don't have a way to flatten
/// [`BlockBody`] into [`Block`].
mod block_rlp {
    use super::*;

    /// Newtype around the transaction list whose [`Decodable`] implementation pre-sizes the
    /// `Vec`, see [`decode_transactions`].
    struct RlpTransactions<T>(Vec<T>);

    impl<T: Decodable> Decodable for RlpTransactions<T> {
        #[inline]
        fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
            decode_transactions(buf).map(Self)
        }
    }

    #[derive(RlpDecodable)]
    #[rlp(trailing(no_gaps))]
    struct Helper<T, H> {
        header: H,
        transactions: RlpTransactions<T>,
        ommers: Vec<H>,
        withdrawals: Option<Withdrawals>,
    }

    #[derive(RlpDecodable)]
    #[rlp(trailing(no_gaps))]
    struct BodyHelper<T, H> {
        transactions: RlpTransactions<T>,
        ommers: Vec<H>,
        withdrawals: Option<Withdrawals>,
    }

    impl<T: Decodable, H: Decodable> Decodable for BlockBody<T, H> {
        fn decode(b: &mut &[u8]) -> alloy_rlp::Result<Self> {
            let BodyHelper { transactions, ommers, withdrawals } = BodyHelper::decode(b)?;
            Ok(Self { transactions: transactions.0, ommers, withdrawals })
        }
    }

    #[derive(RlpEncodable)]
    #[rlp(trailing(no_gaps))]
    pub(crate) struct HelperRef<'a, T, H> {
        pub(crate) header: &'a H,
        pub(crate) transactions: &'a Vec<T>,
        pub(crate) ommers: &'a Vec<H>,
        pub(crate) withdrawals: Option<&'a Withdrawals>,
    }

    impl<'a, T, H> HelperRef<'a, T, H> {
        pub(crate) const fn from_parts(header: &'a H, body: &'a BlockBody<T, H>) -> Self {
            Self {
                header,
                transactions: &body.transactions,
                ommers: &body.ommers,
                withdrawals: body.withdrawals.as_ref(),
            }
        }
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
            Ok(Self {
                header,
                body: BlockBody { transactions: transactions.0, ommers, withdrawals },
            })
        }
    }

    impl<T: Decodable, H: Decodable> Block<T, H> {
        /// Decodes the block from RLP, computing the header hash directly from the RLP bytes.
        ///
        /// This is more efficient than decoding the block and then sealing it, as the header
        /// hash is computed from the raw RLP bytes without re-encoding.
        pub fn decode_sealed(buf: &mut &[u8]) -> alloy_rlp::Result<Sealed<Self>> {
            // Restrict child decoding to the outer block list's declared payload.
            let mut payload = alloy_rlp::Header::decode_bytes(buf, true)?;

            // Decode header and compute hash from raw RLP bytes
            let header_start = payload;
            let header = H::decode(&mut payload)?;
            let header_length = header_start
                .len()
                .checked_sub(payload.len())
                .ok_or(alloy_rlp::Error::InputTooShort)?;
            let header_rlp =
                header_start.get(..header_length).ok_or(alloy_rlp::Error::InputTooShort)?;
            let header_hash = keccak256(header_rlp);

            // Decode remaining body fields
            let transactions = decode_transactions::<T>(&mut payload)?;
            let ommers = Vec::<H>::decode(&mut payload)?;
            let withdrawals =
                if payload.is_empty() { None } else { Some(Decodable::decode(&mut payload)?) };
            if !payload.is_empty() {
                return Err(alloy_rlp::Error::ListLengthMismatch {
                    expected: header_start.len(),
                    got: header_start.len().saturating_sub(payload.len()),
                });
            }

            let block = Self { header, body: BlockBody { transactions, ommers, withdrawals } };

            Ok(Sealed::new_unchecked(block, header_hash))
        }
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
    use alloy_rlp::{Decodable, Encodable};

    #[test]
    fn can_convert_block() {
        let block: Block<Signed<TxLegacy>> = Block::default();
        let _: Block<TxEnvelope> = block.convert_transactions();
    }

    #[test]
    fn decode_sealed_produces_correct_hash() {
        let block: Block<TxEnvelope> = Block::default();
        let expected_hash = block.header.hash_slow();

        let mut encoded = Vec::new();
        block.encode(&mut encoded);

        let mut buf = encoded.as_slice();
        let sealed = Block::<TxEnvelope>::decode_sealed(&mut buf).unwrap();

        assert_eq!(sealed.hash(), expected_hash);
        assert_eq!(*sealed.inner(), block);
    }

    #[test]
    fn header_decode_sealed_produces_correct_hash() {
        let header = Header::default();
        let expected_hash = header.hash_slow();

        let mut encoded = Vec::new();
        header.encode(&mut encoded);

        let mut buf = encoded.as_slice();
        let sealed = Header::decode_sealed(&mut buf).unwrap();

        assert_eq!(sealed.hash(), expected_hash);
        assert_eq!(*sealed.inner(), header);
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_sealed_roundtrip_with_transactions() {
        use crate::{SignableTransaction, TxLegacy};
        use alloy_primitives::{Address, Signature, TxKind, U256};

        let tx = TxLegacy {
            nonce: 1,
            gas_price: 100,
            gas_limit: 21000,
            to: TxKind::Call(Address::ZERO),
            value: U256::from(1000),
            input: Default::default(),
            chain_id: Some(1),
        };
        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = tx.into_signed(sig);
        let envelope: TxEnvelope = signed.into();

        let block = Block {
            header: Header { number: 42, gas_limit: 30_000_000, ..Default::default() },
            body: BlockBody { transactions: vec![envelope], ommers: vec![], withdrawals: None },
        };

        let expected_hash = block.header.hash_slow();

        let mut encoded = Vec::new();
        block.encode(&mut encoded);

        let mut buf = encoded.as_slice();
        let sealed = Block::<TxEnvelope>::decode_sealed(&mut buf).unwrap();

        assert_eq!(sealed.hash(), expected_hash);
        assert_eq!(sealed.header.number, 42);
        assert_eq!(sealed.body.transactions.len(), 1);
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_sealed_rejects_fields_past_outer_rlp_boundary() {
        let block: Block<TxEnvelope> = Block::default();
        let mut encoded = alloy_rlp::encode(&block);

        let mut payload = encoded.as_slice();
        let outer = alloy_rlp::Header::decode(&mut payload).unwrap();
        let header_length = encoded.len() - payload.len();
        assert!(outer.list);
        assert!(outer.payload_length > 56);

        let mut replacement = Vec::with_capacity(header_length);
        alloy_rlp::Header { list: true, payload_length: outer.payload_length - 1 }
            .encode(&mut replacement);
        assert_eq!(replacement.len(), header_length);
        encoded[..header_length].copy_from_slice(&replacement);

        assert!(Block::<TxEnvelope>::decode_sealed(&mut encoded.as_slice()).is_err());
    }

    #[test]
    fn decode_sealed_rejects_decoder_that_expands_input() {
        #[derive(Debug)]
        struct ExpandingHeader;

        impl Decodable for ExpandingHeader {
            fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
                *buf = &[0, 0];
                Ok(Self)
            }
        }

        let mut encoded: &[u8] = &[0xc1, 0x80];
        let result = Block::<TxEnvelope, ExpandingHeader>::decode_sealed(&mut encoded);

        assert!(matches!(result, Err(alloy_rlp::Error::InputTooShort)));
    }

    #[test]
    fn block_body_rejects_present_string_withdrawals() {
        let mut omitted: &[u8] = &[0xc2, 0xc0, 0xc0];
        let body = BlockBody::<TxEnvelope>::decode(&mut omitted).unwrap();
        assert!(body.withdrawals.is_none());
        assert!(omitted.is_empty());

        let mut present_empty: &[u8] = &[0xc3, 0xc0, 0xc0, 0xc0];
        let body = BlockBody::<TxEnvelope>::decode(&mut present_empty).unwrap();
        assert!(body.withdrawals.as_ref().is_some_and(|w| w.is_empty()));
        assert!(present_empty.is_empty());

        let mut present_string: &[u8] = &[0xc3, 0xc0, 0xc0, 0x80];
        assert!(BlockBody::<TxEnvelope>::decode(&mut present_string).is_err());
    }

    #[test]
    fn block_decoders_reject_present_string_withdrawals() {
        fn block_rlp_with_body_fields(body_fields: &[u8]) -> Vec<u8> {
            let mut header = Vec::new();
            Header::default().encode(&mut header);

            let block_header =
                alloy_rlp::Header { list: true, payload_length: header.len() + body_fields.len() };
            let mut out = Vec::with_capacity(block_header.length_with_payload());
            block_header.encode(&mut out);
            out.extend_from_slice(&header);
            out.extend_from_slice(body_fields);
            out
        }

        let omitted = block_rlp_with_body_fields(&[0xc0, 0xc0]);
        assert!(Block::<TxEnvelope>::decode(&mut omitted.as_slice()).is_ok());
        assert!(Block::<TxEnvelope>::decode_sealed(&mut omitted.as_slice()).is_ok());

        let present_empty = block_rlp_with_body_fields(&[0xc0, 0xc0, 0xc0]);
        assert!(Block::<TxEnvelope>::decode(&mut present_empty.as_slice()).is_ok());
        assert!(Block::<TxEnvelope>::decode_sealed(&mut present_empty.as_slice()).is_ok());

        let present_string = block_rlp_with_body_fields(&[0xc0, 0xc0, 0x80]);
        assert!(Block::<TxEnvelope>::decode(&mut present_string.as_slice()).is_err());
        assert!(Block::<TxEnvelope>::decode_sealed(&mut present_string.as_slice()).is_err());
    }

    /// Builds `count` distinct, well formed transaction envelopes.
    fn test_transactions(count: usize) -> Vec<TxEnvelope> {
        use crate::SignableTransaction;
        use alloy_primitives::{Address, Signature, TxKind, U256};

        (0..count as u64)
            .map(|nonce| {
                let tx = TxLegacy {
                    nonce,
                    gas_price: 100,
                    gas_limit: 21_000,
                    to: TxKind::Call(Address::with_last_byte(nonce as u8)),
                    value: U256::from(nonce),
                    input: Default::default(),
                    chain_id: Some(1),
                };
                let sig = Signature::new(U256::from(1), U256::from(2), false);
                tx.into_signed(sig).into()
            })
            .collect()
    }

    #[test]
    fn decode_transactions_matches_vec_decode() {
        for count in [0usize, 1, 2, 8, 9, 64, 2048] {
            let transactions = test_transactions(count);
            let encoded = alloy_rlp::encode(&transactions);

            let mut buf = encoded.as_slice();
            let decoded = decode_transactions::<TxEnvelope>(&mut buf).unwrap();
            assert_eq!(decoded, transactions);
            assert!(buf.is_empty());

            // Identical to what the generic `Vec<T>` decoding produces.
            let mut buf = encoded.as_slice();
            assert_eq!(decoded, Vec::<TxEnvelope>::decode(&mut buf).unwrap());
            assert!(buf.is_empty());
        }
    }

    #[test]
    fn decode_large_block_round_trips() {
        let transactions = test_transactions(2048);
        let block = Block {
            header: Header { number: 42, gas_limit: 30_000_000, ..Default::default() },
            body: BlockBody { transactions, ommers: vec![], withdrawals: None },
        };
        let expected_hash = block.header.hash_slow();
        let encoded = alloy_rlp::encode(&block);

        let mut buf = encoded.as_slice();
        let decoded = Block::<TxEnvelope>::decode(&mut buf).unwrap();
        assert_eq!(decoded, block);
        assert!(buf.is_empty());

        let mut buf = encoded.as_slice();
        let sealed = Block::<TxEnvelope>::decode_sealed(&mut buf).unwrap();
        assert_eq!(sealed.hash(), expected_hash);
        assert_eq!(sealed.body.transactions.len(), 2048);
        assert!(buf.is_empty());

        let body_encoded = alloy_rlp::encode(&block.body);
        let mut buf = body_encoded.as_slice();
        assert_eq!(BlockBody::<TxEnvelope>::decode(&mut buf).unwrap(), block.body);
        assert!(buf.is_empty());
    }

    /// Wraps `payload` in an RLP list header.
    fn rlp_list(payload: &[u8]) -> Vec<u8> {
        let header = alloy_rlp::Header { list: true, payload_length: payload.len() };
        let mut out = Vec::with_capacity(header.length_with_payload());
        header.encode(&mut out);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn decode_transactions_rejects_malformed_input() {
        // Not a list.
        assert!(decode_transactions::<TxEnvelope>(&mut [0x80u8].as_slice()).is_err());

        // A list whose declared payload length exceeds the buffer.
        assert!(decode_transactions::<TxEnvelope>(&mut [0xc4u8, 0x01].as_slice()).is_err());

        // Garbage inside the list payload, both inside and past the sample window.
        for count in [0usize, 1, 32] {
            let mut payload = alloy_rlp::encode(test_transactions(count));
            payload.push(0xff);
            let encoded = rlp_list(&payload);
            assert!(decode_transactions::<TxEnvelope>(&mut encoded.as_slice()).is_err());
            // The generic `Vec<T>` decoding rejects it as well.
            assert!(Vec::<TxEnvelope>::decode(&mut encoded.as_slice()).is_err());
        }

        // A truncated final transaction.
        let payload = alloy_rlp::encode(test_transactions(32));
        let truncated = rlp_list(&payload[..payload.len() - 1]);
        assert!(decode_transactions::<TxEnvelope>(&mut truncated.as_slice()).is_err());
    }

    #[test]
    fn decode_transactions_clamps_pre_allocation() {
        /// A type that is large in memory but encodes to a single byte, i.e. the worst case for
        /// extrapolating an element count from the encoded size.
        #[derive(Debug, PartialEq, Eq)]
        struct Big([u8; 1024]);

        impl Decodable for Big {
            fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
                u8::decode(buf).map(|b| Self([b; 1024]))
            }
        }

        // 4096 single byte items would extrapolate to a 4 MiB reservation from a 4 KiB payload;
        // the clamp keeps it proportional to the encoded size while still decoding correctly.
        let encoded = rlp_list(&[0x01u8; 4096]);
        let decoded = decode_transactions::<Big>(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.len(), 4096);
        assert!(decoded.iter().all(|b| b.0 == [0x01; 1024]));
    }
}

#[cfg(all(test, feature = "arbitrary"))]
mod fuzz_tests {
    use super::*;
    use crate::{EthereumTxEnvelope, TxEip4844};
    use alloy_rlp::Encodable;
    use arbitrary::{Arbitrary, Unstructured};
    use rand::Rng;

    #[test]
    fn fuzz_decode_sealed_block_roundtrip() {
        for _ in 0..10 {
            let mut bytes = [0u8; 1024 * 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let mut u = Unstructured::new(&bytes);

            let block = Block::<EthereumTxEnvelope<TxEip4844>>::arbitrary(&mut u).unwrap();
            let expected_hash = block.header.hash_slow();

            let mut encoded = Vec::new();
            block.encode(&mut encoded);

            let sealed =
                Block::<EthereumTxEnvelope<TxEip4844>>::decode_sealed(&mut encoded.as_slice())
                    .unwrap();
            assert_eq!(sealed.hash(), expected_hash);
            assert_eq!(*sealed.inner(), block);
        }
    }

    #[test]
    fn fuzz_header_decode_sealed_roundtrip() {
        for _ in 0..200 {
            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let mut u = Unstructured::new(&bytes);

            let header = Header::arbitrary(&mut u).unwrap();
            let expected_hash = header.hash_slow();

            let mut encoded = Vec::new();
            header.encode(&mut encoded);

            let mut buf = encoded.as_slice();
            let sealed = Header::decode_sealed(&mut buf).unwrap();

            assert_eq!(sealed.hash(), expected_hash);
            assert_eq!(*sealed.inner(), header);
        }
    }
}
