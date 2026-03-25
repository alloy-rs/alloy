use crate::{BlockTransactions, InclusionInfo};
use alloy_consensus::{BlockHeader, Transaction};
use alloy_eips::BlockNumHash;
use alloy_primitives::{Address, BlockHash, TxHash, B256};
use alloy_serde::WithOtherFields;

/// Error returned when a transaction failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionFailedError {
    /// Hash of the failed transaction.
    pub transaction_hash: TxHash,
}

impl core::fmt::Display for TransactionFailedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Transaction {} failed", self.transaction_hash)
    }
}

impl core::error::Error for TransactionFailedError {}

/// Receipt JSON-RPC response.
pub trait ReceiptResponse {
    /// Address of the created contract, or `None` if the transaction was not a deployment.
    fn contract_address(&self) -> Option<Address>;

    /// Status of the transaction.
    ///
    /// ## Note
    ///
    /// Caution must be taken when using this method for deep-historical
    /// receipts, as it may not accurately reflect the status of the
    /// transaction. The transaction status is not knowable from the receipt
    /// for transactions before [EIP-658].
    fn status(&self) -> bool;

    /// Hash of the block this transaction was included within.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Number of the block this transaction was included within.
    fn block_number(&self) -> Option<u64>;

    /// Returns the [`BlockNumHash`] of the block this transaction was mined in.
    ///
    /// Returns `None` if this transaction is still pending.
    fn block_hash_num(&self) -> Option<BlockNumHash> {
        Some(BlockNumHash::new(self.block_number()?, self.block_hash()?))
    }

    /// Transaction Hash.
    fn transaction_hash(&self) -> TxHash;

    /// Index within the block.
    fn transaction_index(&self) -> Option<u64>;

    /// Gas used by this transaction alone.
    fn gas_used(&self) -> u64;

    /// Effective gas price.
    fn effective_gas_price(&self) -> u128;

    /// Total cost of this transaction = gas_used * effective_gas_price.
    fn cost(&self) -> u128 {
        self.gas_used() as u128 * self.effective_gas_price()
    }

    /// Blob gas used by the eip-4844 transaction.
    fn blob_gas_used(&self) -> Option<u64>;

    /// Blob gas price paid by the eip-4844 transaction.
    fn blob_gas_price(&self) -> Option<u128>;

    /// Address of the sender.
    fn from(&self) -> Address;

    /// Address of the receiver.
    fn to(&self) -> Option<Address>;

    /// Returns the cumulative gas used at this receipt.
    fn cumulative_gas_used(&self) -> u64;

    /// The post-transaction state root (pre Byzantium)
    ///
    /// EIP98 makes this field optional.
    fn state_root(&self) -> Option<B256>;

    /// Ensures the transaction was successful, returning an error if it failed.
    fn ensure_success(&self) -> Result<(), TransactionFailedError> {
        if self.status() {
            Ok(())
        } else {
            Err(TransactionFailedError { transaction_hash: self.transaction_hash() })
        }
    }
}

/// Transaction JSON-RPC response. Aggregates transaction data with its block and signer context.
pub trait TransactionResponse: Transaction {
    /// Hash of the transaction
    #[doc(alias = "transaction_hash")]
    fn tx_hash(&self) -> TxHash;

    /// Returns the hash of the block this transaction was mined in.
    ///
    /// Returns `None` if this transaction is still pending.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Returns the number of the block this transaction was mined in.
    ///
    /// Returns `None` if this transaction is still pending.
    fn block_number(&self) -> Option<u64>;

    /// Returns the [`BlockNumHash`] of the block this transaction was mined in.
    ///
    /// Returns `None` if this transaction is still pending.
    fn block_hash_num(&self) -> Option<BlockNumHash> {
        Some(BlockNumHash::new(self.block_number()?, self.block_hash()?))
    }

    /// Transaction Index
    fn transaction_index(&self) -> Option<u64>;

    /// Sender of the transaction
    fn from(&self) -> Address;

    /// Gas Price, this is the RPC format for `max_fee_per_gas`, pre-eip-1559.
    fn gas_price(&self) -> Option<u128> {
        if self.ty() < 2 {
            return Some(Transaction::max_fee_per_gas(self));
        }
        None
    }

    /// Max BaseFeePerGas the user is willing to pay. For pre-eip-1559 transactions, the field
    /// label `gas_price` is used instead.
    fn max_fee_per_gas(&self) -> Option<u128> {
        if self.ty() < 2 {
            return None;
        }
        Some(Transaction::max_fee_per_gas(self))
    }

    /// Transaction type format for RPC. This field is included since eip-2930.
    fn transaction_type(&self) -> Option<u8> {
        match self.ty() {
            0 => None,
            ty => Some(ty),
        }
    }

    /// Returns the [`InclusionInfo`] if the transaction has been included.
    ///
    /// Returns `None` if this transaction is still pending (missing block number, hash, or index).
    fn inclusion_info(&self) -> Option<InclusionInfo> {
        Some(InclusionInfo {
            block_hash: self.block_hash()?,
            block_number: self.block_number()?,
            transaction_index: self.transaction_index()?,
        })
    }
}

/// Header JSON-RPC response.
pub trait HeaderResponse: BlockHeader {
    /// Block hash
    fn hash(&self) -> BlockHash;
}

/// Block JSON-RPC response.
pub trait BlockResponse {
    /// Header type
    type Header;
    /// Transaction type
    type Transaction: TransactionResponse;

    /// Block header
    fn header(&self) -> &Self::Header;

    /// Block transactions
    fn transactions(&self) -> &BlockTransactions<Self::Transaction>;

    /// Mutable reference to block transactions
    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction>;

    /// Returns the `other` field from `WithOtherFields` type.
    fn other_fields(&self) -> Option<&alloy_serde::OtherFields> {
        None
    }
}

impl<T: TransactionResponse> TransactionResponse for WithOtherFields<T> {
    fn tx_hash(&self) -> TxHash {
        self.inner.tx_hash()
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.inner.block_hash()
    }

    fn block_number(&self) -> Option<u64> {
        self.inner.block_number()
    }

    fn transaction_index(&self) -> Option<u64> {
        self.inner.transaction_index()
    }

    fn from(&self) -> Address {
        self.inner.from()
    }
}

impl<T: ReceiptResponse> ReceiptResponse for WithOtherFields<T> {
    fn contract_address(&self) -> Option<Address> {
        self.inner.contract_address()
    }

    fn status(&self) -> bool {
        self.inner.status()
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.inner.block_hash()
    }

    fn block_number(&self) -> Option<u64> {
        self.inner.block_number()
    }

    fn transaction_hash(&self) -> TxHash {
        self.inner.transaction_hash()
    }

    fn transaction_index(&self) -> Option<u64> {
        self.inner.transaction_index()
    }

    fn gas_used(&self) -> u64 {
        self.inner.gas_used()
    }

    fn effective_gas_price(&self) -> u128 {
        self.inner.effective_gas_price()
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.inner.blob_gas_used()
    }

    fn blob_gas_price(&self) -> Option<u128> {
        self.inner.blob_gas_price()
    }

    fn from(&self) -> Address {
        self.inner.from()
    }

    fn to(&self) -> Option<Address> {
        self.inner.to()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.inner.cumulative_gas_used()
    }

    fn state_root(&self) -> Option<B256> {
        self.inner.state_root()
    }
}

impl<T: BlockResponse> BlockResponse for WithOtherFields<T> {
    type Header = T::Header;
    type Transaction = T::Transaction;

    fn header(&self) -> &Self::Header {
        self.inner.header()
    }

    fn transactions(&self) -> &BlockTransactions<Self::Transaction> {
        self.inner.transactions()
    }

    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction> {
        self.inner.transactions_mut()
    }

    fn other_fields(&self) -> Option<&alloy_serde::OtherFields> {
        Some(&self.other)
    }
}

impl<T: HeaderResponse> HeaderResponse for WithOtherFields<T> {
    fn hash(&self) -> BlockHash {
        self.inner.hash()
    }
}
