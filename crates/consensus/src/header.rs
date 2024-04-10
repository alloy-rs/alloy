use crate::Sealable;
use alloy_eips::{
    eip1559::{calc_next_block_base_fee, BaseFeeParams},
    eip4844::{calc_blob_gasprice, calc_excess_blob_gas},
};
use alloy_primitives::{b256, keccak256, Address, BlockNumber, Bloom, Bytes, B256, B64, U256};
use alloy_rlp::{
    length_of_length, Buf, BufMut, Decodable, Encodable, EMPTY_LIST_CODE, EMPTY_STRING_CODE,
};
use core::mem;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Ommer root of empty list.
pub const EMPTY_OMMER_ROOT_HASH: B256 =
    b256!("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347");

/// Root hash of an empty trie.
pub const EMPTY_ROOT_HASH: B256 =
    b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");

/// Ethereum Block header
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Header {
    /// The Keccak 256-bit hash of the parent
    /// block’s header, in its entirety; formally Hp.
    pub parent_hash: B256,
    /// The Keccak 256-bit hash of the ommers list portion of this block; formally Ho.
    pub ommers_hash: B256,
    /// The 160-bit address to which all fees collected from the successful mining of this block
    /// be transferred; formally Hc.
    pub beneficiary: Address,
    /// The Keccak 256-bit hash of the root node of the state trie, after all transactions are
    /// executed and finalisations applied; formally Hr.
    pub state_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with each
    /// transaction in the transactions list portion of the block; formally Ht.
    pub transactions_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with the receipts
    /// of each transaction in the transactions list portion of the block; formally He.
    pub receipts_root: B256,
    /// The Keccak 256-bit hash of the withdrawals list portion of this block.
    /// <https://eips.ethereum.org/EIPS/eip-4895>
    pub withdrawals_root: Option<B256>,
    /// The Bloom filter composed from indexable information (logger address and log topics)
    /// contained in each log entry from the receipt of each transaction in the transactions list;
    /// formally Hb.
    pub logs_bloom: Bloom,
    /// A scalar value corresponding to the difficulty level of this block. This can be calculated
    /// from the previous block’s difficulty level and the timestamp; formally Hd.
    pub difficulty: U256,
    /// A scalar value equal to the number of ancestor blocks. The genesis block has a number of
    /// zero; formally Hi.
    pub number: BlockNumber,
    /// A scalar value equal to the current limit of gas expenditure per block; formally Hl.
    pub gas_limit: u128,
    /// A scalar value equal to the total gas used in transactions in this block; formally Hg.
    pub gas_used: u128,
    /// A scalar value equal to the reasonable output of Unix’s time() at this block’s inception;
    /// formally Hs.
    pub timestamp: u64,
    /// A 256-bit hash which, combined with the
    /// nonce, proves that a sufficient amount of computation has been carried out on this block;
    /// formally Hm.
    pub mix_hash: B256,
    /// A 64-bit value which, combined with the mixhash, proves that a sufficient amount of
    /// computation has been carried out on this block; formally Hn.
    pub nonce: B64,
    /// A scalar representing EIP1559 base fee which can move up or down each block according
    /// to a formula which is a function of gas used in parent block and gas target
    /// (block gas limit divided by elasticity multiplier) of parent block.
    /// The algorithm results in the base fee per gas increasing when blocks are
    /// above the gas target, and decreasing when blocks are below the gas target. The base fee per
    /// gas is burned.
    pub base_fee_per_gas: Option<u128>,
    /// The total amount of blob gas consumed by the transactions within the block, added in
    /// EIP-4844.
    pub blob_gas_used: Option<u128>,
    /// A running total of blob gas consumed in excess of the target, prior to the block. Blocks
    /// with above-target blob gas consumption increase this value, blocks with below-target blob
    /// gas consumption decrease it (bounded at 0). This was added in EIP-4844.
    pub excess_blob_gas: Option<u128>,
    /// The hash of the parent beacon block's root is included in execution blocks, as proposed by
    /// EIP-4788.
    ///
    /// This enables trust-minimized access to consensus state, supporting staking pools, bridges,
    /// and more.
    ///
    /// The beacon roots contract handles root storage, enhancing Ethereum's functionalities.
    pub parent_beacon_block_root: Option<B256>,
    /// An arbitrary byte array containing data relevant to this block. This must be 32 bytes or
    /// fewer; formally Hx.
    pub extra_data: Bytes,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            parent_hash: Default::default(),
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: Default::default(),
            state_root: EMPTY_ROOT_HASH,
            transactions_root: EMPTY_ROOT_HASH,
            receipts_root: EMPTY_ROOT_HASH,
            logs_bloom: Default::default(),
            difficulty: Default::default(),
            number: 0,
            gas_limit: 0,
            gas_used: 0,
            timestamp: 0,
            extra_data: Default::default(),
            mix_hash: Default::default(),
            nonce: B64::ZERO,
            base_fee_per_gas: None,
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
        }
    }
}

impl Sealable for Header {
    fn hash(&self) -> B256 {
        self.hash_slow()
    }
}

impl Header {
    // TODO: re-enable

    // /// Returns the parent block's number and hash
    // pub fn parent_num_hash(&self) -> BlockNumHash {
    //     BlockNumHash { number: self.number.saturating_sub(1), hash: self.parent_hash }
    // }

    /// Heavy function that will calculate hash of data and will *not* save the change to metadata.
    ///
    /// Use [`Header::seal_slow`] and unlock if you need the hash to be persistent.
    pub fn hash_slow(&self) -> B256 {
        let mut out = Vec::<u8>::new();
        self.encode(&mut out);
        keccak256(&out)
    }

    /// Checks if the header is empty - has no transactions and no ommers
    pub fn is_empty(&self) -> bool {
        let txs_and_ommers_empty = self.transaction_root_is_empty() && self.ommers_hash_is_empty();
        if let Some(withdrawals_root) = self.withdrawals_root {
            txs_and_ommers_empty && withdrawals_root == EMPTY_ROOT_HASH
        } else {
            txs_and_ommers_empty
        }
    }

    /// Check if the ommers hash equals to empty hash list.
    pub fn ommers_hash_is_empty(&self) -> bool {
        self.ommers_hash == EMPTY_OMMER_ROOT_HASH
    }

    /// Check if the transaction root equals to empty root.
    pub fn transaction_root_is_empty(&self) -> bool {
        self.transactions_root == EMPTY_ROOT_HASH
    }

    // TODO: re-enable

    // /// Converts all roots in the header to a [BlockBodyRoots] struct.
    // pub fn body_roots(&self) -> BlockBodyRoots {
    //     BlockBodyRoots {
    //         tx_root: self.transactions_root,
    //         ommers_hash: self.ommers_hash,
    //         withdrawals_root: self.withdrawals_root,
    //     }
    // }

    /// Returns the blob fee for _this_ block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None
    pub fn blob_fee(&self) -> Option<u128> {
        self.excess_blob_gas.map(calc_blob_gasprice)
    }

    /// Returns the blob fee for the next block according to the EIP-4844 spec.
    ///
    /// Returns `None` if `excess_blob_gas` is None.
    ///
    /// See also [Self::next_block_excess_blob_gas]
    pub fn next_block_blob_fee(&self) -> Option<u128> {
        self.next_block_excess_blob_gas().map(calc_blob_gasprice)
    }

    /// Calculate base fee for next block according to the EIP-1559 spec.
    ///
    /// Returns a `None` if no base fee is set, no EIP-1559 support
    pub fn next_block_base_fee(&self, base_fee_params: BaseFeeParams) -> Option<u128> {
        Some(calc_next_block_base_fee(
            self.gas_used,
            self.gas_limit,
            self.base_fee_per_gas?,
            base_fee_params,
        ))
    }

    /// Calculate excess blob gas for the next block according to the EIP-4844
    /// spec.
    ///
    /// Returns a `None` if no excess blob gas is set, no EIP-4844 support
    pub fn next_block_excess_blob_gas(&self) -> Option<u128> {
        Some(calc_excess_blob_gas(self.excess_blob_gas?, self.blob_gas_used?))
    }

    /// Calculate a heuristic for the in-memory size of the [Header].
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<B256>() + // parent hash
        mem::size_of::<B256>() + // ommers hash
        mem::size_of::<Address>() + // beneficiary
        mem::size_of::<B256>() + // state root
        mem::size_of::<B256>() + // transactions root
        mem::size_of::<B256>() + // receipts root
        mem::size_of::<Option<B256>>() + // withdrawals root
        mem::size_of::<Bloom>() + // logs bloom
        mem::size_of::<U256>() + // difficulty
        mem::size_of::<BlockNumber>() + // number
        mem::size_of::<u128>() + // gas limit
        mem::size_of::<u128>() + // gas used
        mem::size_of::<u64>() + // timestamp
        mem::size_of::<B256>() + // mix hash
        mem::size_of::<u64>() + // nonce
        mem::size_of::<Option<u128>>() + // base fee per gas
        mem::size_of::<Option<u128>>() + // blob gas used
        mem::size_of::<Option<u128>>() + // excess blob gas
        mem::size_of::<Option<B256>>() + // parent beacon block root
        self.extra_data.len() // extra data
    }

    fn header_payload_length(&self) -> usize {
        let mut length = 0;
        length += self.parent_hash.length();
        length += self.ommers_hash.length();
        length += self.beneficiary.length();
        length += self.state_root.length();
        length += self.transactions_root.length();
        length += self.receipts_root.length();
        length += self.logs_bloom.length();
        length += self.difficulty.length();
        length += U256::from(self.number).length();
        length += U256::from(self.gas_limit).length();
        length += U256::from(self.gas_used).length();
        length += self.timestamp.length();
        length += self.extra_data.length();
        length += self.mix_hash.length();
        length += self.nonce.length();

        if let Some(base_fee) = self.base_fee_per_gas {
            length += U256::from(base_fee).length();
        } else if self.withdrawals_root.is_some()
            || self.blob_gas_used.is_some()
            || self.excess_blob_gas.is_some()
            || self.parent_beacon_block_root.is_some()
        {
            length += 1; // EMPTY LIST CODE
        }

        if let Some(root) = self.withdrawals_root {
            length += root.length();
        } else if self.blob_gas_used.is_some()
            || self.excess_blob_gas.is_some()
            || self.parent_beacon_block_root.is_some()
        {
            length += 1; // EMPTY STRING CODE
        }

        if let Some(blob_gas_used) = self.blob_gas_used {
            length += U256::from(blob_gas_used).length();
        } else if self.excess_blob_gas.is_some() || self.parent_beacon_block_root.is_some() {
            length += 1; // EMPTY LIST CODE
        }

        if let Some(excess_blob_gas) = self.excess_blob_gas {
            length += U256::from(excess_blob_gas).length();
        } else if self.parent_beacon_block_root.is_some() {
            length += 1; // EMPTY LIST CODE
        }

        // Encode parent beacon block root length. If new fields are added, the above pattern will
        // need to be repeated and placeholder length added. Otherwise, it's impossible to
        // tell _which_ fields are missing. This is mainly relevant for contrived cases
        // where a header is created at random, for example:
        //  * A header is created with a withdrawals root, but no base fee. Shanghai blocks are
        //    post-London, so this is technically not valid. However, a tool like proptest would
        //    generate a block like this.
        if let Some(parent_beacon_block_root) = self.parent_beacon_block_root {
            length += parent_beacon_block_root.length();
        }

        length
    }
}

impl Encodable for Header {
    fn encode(&self, out: &mut dyn BufMut) {
        let list_header =
            alloy_rlp::Header { list: true, payload_length: self.header_payload_length() };
        list_header.encode(out);
        self.parent_hash.encode(out);
        self.ommers_hash.encode(out);
        self.beneficiary.encode(out);
        self.state_root.encode(out);
        self.transactions_root.encode(out);
        self.receipts_root.encode(out);
        self.logs_bloom.encode(out);
        self.difficulty.encode(out);
        U256::from(self.number).encode(out);
        U256::from(self.gas_limit).encode(out);
        U256::from(self.gas_used).encode(out);
        self.timestamp.encode(out);
        self.extra_data.encode(out);
        self.mix_hash.encode(out);
        self.nonce.encode(out);

        // Encode base fee. Put empty list if base fee is missing,
        // but withdrawals root is present.
        if let Some(ref base_fee) = self.base_fee_per_gas {
            U256::from(*base_fee).encode(out);
        } else if self.withdrawals_root.is_some()
            || self.blob_gas_used.is_some()
            || self.excess_blob_gas.is_some()
            || self.parent_beacon_block_root.is_some()
        {
            out.put_u8(EMPTY_LIST_CODE);
        }

        // Encode withdrawals root. Put empty string if withdrawals root is missing,
        // but blob gas used is present.
        if let Some(ref root) = self.withdrawals_root {
            root.encode(out);
        } else if self.blob_gas_used.is_some()
            || self.excess_blob_gas.is_some()
            || self.parent_beacon_block_root.is_some()
        {
            out.put_u8(EMPTY_STRING_CODE);
        }

        // Encode blob gas used. Put empty list if blob gas used is missing,
        // but excess blob gas is present.
        if let Some(ref blob_gas_used) = self.blob_gas_used {
            U256::from(*blob_gas_used).encode(out);
        } else if self.excess_blob_gas.is_some() || self.parent_beacon_block_root.is_some() {
            out.put_u8(EMPTY_LIST_CODE);
        }

        // Encode excess blob gas. Put empty list if excess blob gas is missing,
        // but parent beacon block root is present.
        if let Some(ref excess_blob_gas) = self.excess_blob_gas {
            U256::from(*excess_blob_gas).encode(out);
        } else if self.parent_beacon_block_root.is_some() {
            out.put_u8(EMPTY_LIST_CODE);
        }

        // Encode parent beacon block root. If new fields are added, the above pattern will need to
        // be repeated and placeholders added. Otherwise, it's impossible to tell _which_
        // fields are missing. This is mainly relevant for contrived cases where a header is
        // created at random, for example:
        //  * A header is created with a withdrawals root, but no base fee. Shanghai blocks are
        //    post-London, so this is technically not valid. However, a tool like proptest would
        //    generate a block like this.
        if let Some(ref parent_beacon_block_root) = self.parent_beacon_block_root {
            parent_beacon_block_root.encode(out);
        }
    }

    fn length(&self) -> usize {
        let mut length = 0;
        length += self.header_payload_length();
        length += length_of_length(length);
        length
    }
}

impl Decodable for Header {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let rlp_head = alloy_rlp::Header::decode(buf)?;
        if !rlp_head.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let started_len = buf.len();
        let mut this = Self {
            parent_hash: Decodable::decode(buf)?,
            ommers_hash: Decodable::decode(buf)?,
            beneficiary: Decodable::decode(buf)?,
            state_root: Decodable::decode(buf)?,
            transactions_root: Decodable::decode(buf)?,
            receipts_root: Decodable::decode(buf)?,
            logs_bloom: Decodable::decode(buf)?,
            difficulty: Decodable::decode(buf)?,
            number: u64::decode(buf)?,
            gas_limit: u128::decode(buf)?,
            gas_used: u128::decode(buf)?,
            timestamp: Decodable::decode(buf)?,
            extra_data: Decodable::decode(buf)?,
            mix_hash: Decodable::decode(buf)?,
            nonce: B64::decode(buf)?,
            base_fee_per_gas: None,
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
        };

        if started_len - buf.len() < rlp_head.payload_length {
            if buf.first().map(|b| *b == EMPTY_LIST_CODE).unwrap_or_default() {
                buf.advance(1)
            } else {
                this.base_fee_per_gas = Some(U256::decode(buf)?.to::<u128>());
            }
        }

        // Withdrawals root for post-shanghai headers
        if started_len - buf.len() < rlp_head.payload_length {
            if buf.first().map(|b| *b == EMPTY_STRING_CODE).unwrap_or_default() {
                buf.advance(1)
            } else {
                this.withdrawals_root = Some(Decodable::decode(buf)?);
            }
        }

        // Blob gas used and excess blob gas for post-cancun headers
        if started_len - buf.len() < rlp_head.payload_length {
            if buf.first().map(|b| *b == EMPTY_LIST_CODE).unwrap_or_default() {
                buf.advance(1)
            } else {
                this.blob_gas_used = Some(U256::decode(buf)?.to::<u128>());
            }
        }

        if started_len - buf.len() < rlp_head.payload_length {
            if buf.first().map(|b| *b == EMPTY_LIST_CODE).unwrap_or_default() {
                buf.advance(1)
            } else {
                this.excess_blob_gas = Some(U256::decode(buf)?.to::<u128>());
            }
        }

        // Decode parent beacon block root. If new fields are added, the above pattern will need to
        // be repeated and placeholders decoded. Otherwise, it's impossible to tell _which_
        // fields are missing. This is mainly relevant for contrived cases where a header is
        // created at random, for example:
        //  * A header is created with a withdrawals root, but no base fee. Shanghai blocks are
        //    post-London, so this is technically not valid. However, a tool like proptest would
        //    generate a block like this.
        if started_len - buf.len() < rlp_head.payload_length {
            this.parent_beacon_block_root = Some(B256::decode(buf)?);
        }

        let consumed = started_len - buf.len();
        if consumed != rlp_head.payload_length {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        Ok(this)
    }
}
